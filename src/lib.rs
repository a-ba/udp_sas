//! This crate provides an [extension trait] for `std::net::UdpSocket` that supports source address
//! selection for outgoing UDP datagrams. This is useful for implementing a UDP server that binds
//! multiple network interfaces.
//! 
//! The implementation relies on socket options [`IP_PKTINFO`] \(for IPv4) and [`IPV6_RECVPKTINFO`]
//! \(for IPv6).
//! 
//! [extension trait]:      trait.UdpSas.html
//! [`IP_PKTINFO`]:         http://man7.org/linux/man-pages/man7/ip.7.html      
//! [`IPV6_RECVPKTINFO`]:   http://man7.org/linux/man-pages/man7/ipv6.7.html
//! 
//! 
//! ```
//! use std::net::{UdpSocket,SocketAddr};
//! use udp_sas::UdpSas;
//! 
//! fn main() {
//!     demo().unwrap();
//! }
//! fn demo() -> std::io::Result<()>
//! {
//!     let mut buf = [0u8;128];
//! 
//!     // Create the server socket and bind it to 0.0.0.0:30012
//!     //
//!     // Note: we will use 127.0.0.23 as source/destination address
//!     //       for our datagrams (to demonstrate the crate features)
//!     //
//!     let srv = UdpSocket::bind_sas("0.0.0.0:30012".parse::<SocketAddr>().unwrap())?;
//!     let srv_addr = "127.0.0.23:30012".parse().unwrap();
//! 
//!     // Create the client socket and bind it to an anonymous port
//!     //
//!     // Note: we will use 127.0.0.45 as source/destination address
//!     //       for our datagrams (to demonstrate the crate features)
//!     //
//!     let cli = UdpSocket::bind_sas("0.0.0.0:0".parse::<SocketAddr>().unwrap())?;
//!     let cli_addr = SocketAddr::new(
//!         "127.0.0.45".parse().unwrap(),
//!         cli.local_addr().unwrap().port());
//!     assert_ne!(cli_addr.port(), 0);
//!     
//! 
//!     // send a request (msg1) from the client to the server
//!     let msg1 = "What do you get if you multiply six by nine?";
//!     let nb = cli.send_sas(msg1.as_bytes(), &srv_addr, &cli_addr.ip())?;
//!     assert_eq!(nb, msg1.as_bytes().len());
//! 
//!     // receive the request on the server
//!     let (nb, peer, local) = srv.recv_sas(&mut buf)?;
//!     assert_eq!(peer,  cli_addr);
//!     assert_eq!(local, srv_addr.ip());
//!     assert_eq!(nb,          msg1.as_bytes().len());
//!     assert_eq!(&buf[0..nb], msg1.as_bytes());
//!           
//!     // send a reply (msg2) from the server to the client
//!     let msg2 = "Forty-two";
//!     let nb = srv.send_sas(msg2.as_bytes(), &peer, &local)?;
//!     assert_eq!(nb, msg2.as_bytes().len());
//! 
//!     // receive the reply on the client
//!     let (nb, peer, local) = cli.recv_sas(&mut buf)?;
//!     assert_eq!(peer,  srv_addr);
//!     assert_eq!(local, cli_addr.ip());
//!     assert_eq!(nb,          msg2.as_bytes().len());
//!     assert_eq!(&buf[0..nb], msg2.as_bytes());
//!     
//!     Ok(())
//! }
//! ```


extern crate libc;
extern crate os_socketaddr;

use std::io;
use std::net::{UdpSocket,ToSocketAddrs, SocketAddr, IpAddr};
use std::os::unix::io::{AsRawFd,RawFd};

use os_socketaddr::OsSocketAddr;

// C glue
#[link(name="rust_udp_sas", kind="static")]
extern {
    static udp_sas_IPV6_RECVPKTINFO: libc::c_int;
    static udp_sas_IP_PKTINFO: libc::c_int;
    fn udp_sas_recv(sock: libc::c_int, 
                 buf: *mut u8, buf_len: libc::size_t, flags: libc::c_int,
                 src: *mut libc::sockaddr, src_len: libc::socklen_t,
                 dst: *mut libc::sockaddr, dst_len: libc::socklen_t,
                 ) -> libc::ssize_t;

    fn udp_sas_send(sock: libc::c_int, 

                 buf: *const u8, buf_len: libc::size_t, flags: libc::c_int,
                 src: *const libc::sockaddr, src_len: libc::socklen_t,
                 dst: *const libc::sockaddr, dst_len: libc::socklen_t,
                 ) -> libc::ssize_t;
}
use self::udp_sas_IP_PKTINFO as IP_PKTINFO;
use self::udp_sas_IPV6_RECVPKTINFO as IPV6_RECVPKTINFO;

macro_rules! try_io {
    ($x:expr) => {
        match $x {
            -1 => {return Err(io::Error::last_os_error());},
            x  => x
            }}
}

fn getsockopt<T>(socket: RawFd, level: libc::c_int, name: libc::c_int, value: &mut T)
    -> io::Result<libc::socklen_t>
{
    unsafe {
        let mut len = std::mem::size_of::<T>() as libc::socklen_t;
        try_io!(libc::getsockopt(socket, level, name,
                                 value as *mut T as *mut libc::c_void,
                                 &mut len));
        Ok(len)
    }
}
fn setsockopt<T>(socket: RawFd, level: libc::c_int, name: libc::c_int, value: &T)
    -> io::Result<()>
{
    unsafe {
        try_io!(libc::setsockopt(socket, level, name,
                                 value as *const T as *const libc::c_void,
                                 std::mem::size_of::<T>() as libc::socklen_t));
        Ok(())
    }
}

/// enable IP_PKTINFO/IPV6_RECVPKTINFO on a socket
pub fn set_pktinfo(socket: RawFd) -> io::Result<()>
{
    unsafe {
        let mut domain = libc::c_int::default();
        getsockopt(socket, libc::SOL_SOCKET, libc::SO_DOMAIN, &mut domain)?;

        let (level, option) = match domain {
            libc::AF_INET  => (libc::IPPROTO_IP,   IP_PKTINFO),
            libc::AF_INET6 => (libc::IPPROTO_IPV6, IPV6_RECVPKTINFO),
            _ => { return Err(io::Error::new(io::ErrorKind::Other, "not an inet socket")); }
        };

        setsockopt(socket, level, option, &(1 as libc::c_int))
    }
}


/// Receive a datagram (low-level function)
/// 
/// Parameters
/// 
/// * `buf`: buffer for storing the payload
/// 
/// Returns a tuple containing:
/// 
///   * the size of the payload
///   * the source socket address (peer)
///   * the destination ip address (local)
/// 
/// Note: the source (peer) and destination (local) addresses may not be present in the result if
/// the underlying socket does not provide them.
pub fn recv_sas(socket: RawFd, buf: &mut [u8])
    -> io::Result<(usize, Option<SocketAddr>, Option<IpAddr>)>
{
    let mut src = OsSocketAddr::new();
    let mut dst = OsSocketAddr::new();
    
    let nb = {
        unsafe {udp_sas_recv(socket,
                             buf.as_mut_ptr(), buf.len(), 0,
                             src.as_mut_ptr(), src.capacity() as libc::socklen_t,
                             dst.as_mut_ptr(), dst.capacity() as libc::socklen_t,
                             )}
    };

    if nb < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok((nb as usize, src.into(), dst.into_addr().map(|addr| addr.ip())))
    }
}

/// Send datagram (low-level function)
/// 
/// Return the size of the sent payload
/// 
/// Note: the source (local) and destination (target) addresses are optional.
pub fn send_sas(socket: RawFd, buf: &[u8], target: Option<&SocketAddr>, local: Option<&IpAddr>)
    -> io::Result<usize>
{
    let src = match local {
        None     => OsSocketAddr::new(),
        Some(ip) => SocketAddr::new(*ip, 0).into()
    };
    let dst : OsSocketAddr = target.map(|a|*a).into();

    let nb = unsafe { udp_sas_send(socket,
                                   buf.as_ptr(), buf.len(), 0,
                                   src.as_ptr(), src.len() as libc::socklen_t,
                                   dst.as_ptr(), dst.len() as libc::socklen_t)};
    if nb < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(nb as usize)
    }
}

/// An extension trait to support source address selection in `std::net::UdpSocket`
/// 
/// See [module level][mod] documentation for more details.
/// 
/// [mod]: index.html
/// 
pub trait UdpSas : Sized
{
    /// Creates a UDP socket from the given address.
    ///
    /// The address type can be any implementor of [`ToSocketAddrs`] trait. See
    /// its documentation for concrete examples.
    ///
    /// [`ToSocketAddrs`]: https://doc.rust-lang.org/nightly/std/net/addr/trait.ToSocketAddrs.html
    ///
    /// The new socket is configured with the `IP_PKTINFO` or `IPV6_RECVPKTINFO` option enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::net::UdpSocket;
    /// use udp_sas::UdpSas;
    ///
    /// let socket = UdpSocket::bind_sas("127.0.0.1:34254").expect("couldn't bind to address");
    /// ```
    fn bind_sas<A: ToSocketAddrs>(addr: A) -> io::Result<Self>;


    /// Sends a datagram to the given `target` address and use the `local` address as its
    /// source.
    /// 
    /// On success, returns the number of bytes written.
    fn send_sas(&self, buf: &[u8], target: &SocketAddr, local: &IpAddr) -> io::Result<usize>;

    /// Receive a datagram
    /// 
    /// On success, returns a tuple `(nb, source, local)` containing the number of bytes read, the
    /// source socket address (peer address), and the destination ip address (local address).
    /// 
    fn recv_sas(&self, buf: &mut[u8]) -> io::Result<(usize, SocketAddr, IpAddr)>;
}

impl UdpSas for UdpSocket
{
    fn bind_sas<A: ToSocketAddrs>(addr: A) -> io::Result<UdpSocket> {
        let sock = UdpSocket::bind(addr)?;
        set_pktinfo(sock.as_raw_fd())?;
        Ok(sock)
    }

    fn send_sas(&self, buf: &[u8], target: &SocketAddr, local: &IpAddr) -> io::Result<usize>
    {
        send_sas(self.as_raw_fd(), buf, Some(target), Some(local))
    }

    fn recv_sas(&self, buf: &mut[u8]) -> io::Result<(usize, SocketAddr, IpAddr)>
    {
        let (nb, src, local) = recv_sas(self.as_raw_fd(), buf)?;
        match (src, local) {
            (Some(src), Some(local)) => Ok((nb, src, local)),
            (None, _) => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "local address not available (IP_PKTINFO/IPV6_RECVPKTINFO may not be enabled on the socket)")),
            (_, None) => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "source address not available (maybe the socket is connected)"
                    )),
        }
    }
}


