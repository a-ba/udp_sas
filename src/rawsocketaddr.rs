
use ::std;
use ::std::net::SocketAddr;
use ::libc;

/// A type for handling conversions between std::net::SocketAddr and libc::{sockaddr_in,sockaddr_in6}
/// 
/// This type contains just a buffer enough big to hold a `libc::sockaddr_in` or
/// `libc::sockaddr_in6` struct.
/// 
/// Its content can be arbitrary written using `.as_mut()`. Then a call to `.into_addr()` will
/// attempt to convert it into `std::net::SocketAddr`.
///
#[derive(Copy,Clone)]
pub struct RawSocketAddr
{
    sa6: libc::sockaddr_in6
}

#[allow(dead_code)]
impl RawSocketAddr {

    /// Create a new empty socket address
    pub fn new() -> Self
    {
        RawSocketAddr{sa6: unsafe { std::mem::zeroed() }}
    }

    /// Create a new socket address from a raw slice
    /// 
    /// This function will fill the internal buffer with the slice pointed by (`ptr`, `len`). If
    /// `len` is greater than the buffer size then the input is truncated.
    /// 
    /// # Panics
    /// 
    /// Panics if `len` is bigger that the size of `libc::sockaddr_in6`
    /// 
    pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> Self
    {
        let mut raw = RawSocketAddr::new();
        assert!(len <= std::mem::size_of_val(&raw.sa6));
        raw.as_mut()[..len].copy_from_slice(std::slice::from_raw_parts(ptr, len));
        raw
    }

    /// Create a new socket address from a `std::net::SocketAddr` object
    pub fn from(addr: Option<&SocketAddr>) -> Self
    {
        RawSocketAddr{sa6: unsafe {
            match addr {
                None => std::mem::zeroed(),
                Some(&SocketAddr::V4(addr)) => {
                    let mut sa6 = std::mem::uninitialized();
                    *(&mut sa6 as *mut _ as *mut _) = addr;
                    sa6
                },
                Some(&SocketAddr::V6(addr)) =>
                    *(&addr as *const _ as *const _),
            }
        }}
    }

    /// Attempt to convert the internal buffer into a `std::net::SocketAddr` object
    /// 
    /// The internal buffer is assumed to be a `libc::sockaddr`.
    /// 
    /// If the value of `.sa_family` resolves to `AF_INET` or `AF_INET6` then the buffer is
    /// converted into `SocketAddr`, otherwise the function returns None.
    /// 
    pub fn into_addr(self) -> Option<SocketAddr>
    {
        self.into()
    }

    /// Return the length of the address
    /// 
    /// The result depends on the value of `.sa_family` in the internal buffer:
    /// * `AF_INET`  -> the size of `sockaddr_in`
    /// * `AF_INET6` -> the size of `sockaddr_in6`
    /// * *other* -> 0
    /// 
    pub fn len(&self) -> usize
    {
        match self.sa6.sin6_family as i32 {
            libc::AF_INET  => std::mem::size_of::<libc::sockaddr_in >(),
            libc::AF_INET6 => std::mem::size_of::<libc::sockaddr_in6>(),
            _ => 0
        }
    }

    /// Return the size of the internal buffer
    pub fn capacity(&self) -> usize
    {
        std::mem::size_of::<libc::sockaddr_in6>()
    }

    /// Get a pointer to the internal buffer
    pub fn as_ptr(&self) -> *const libc::sockaddr {
        &self.sa6 as *const _ as *const _
    }

    /// Get a mutable pointer to the internal buffer
    pub fn as_mut_ptr(&mut self) -> *mut libc::sockaddr {
        &mut self.sa6 as *mut _ as *mut _
    }

}

impl AsRef<[u8]> for RawSocketAddr
{
    /// Get the internal buffer as a byte slice
    /// 
    /// Note: the actual length of slice depends on the value of `.sa_family` (see `.len()`)
    /// 
    fn as_ref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(&self.sa6 as *const _ as *const u8, self.len())
        }
    }
}

impl AsMut<[u8]> for RawSocketAddr
{
    /// Get the internal buffer as a mutable slice
    fn as_mut(&mut self) -> &mut[u8] {
        unsafe {
            std::slice::from_raw_parts_mut(&mut self.sa6 as *mut _ as *mut u8, self.capacity())
        }
    }
}

impl Into<Option<SocketAddr>> for RawSocketAddr
{
    /// Attempt to convert the internal buffer into a `std::net::SocketAddr` object
    /// 
    /// The internal buffer is assumed to be a `libc::sockaddr`.
    /// 
    /// If the value of `.sa_family` resolves to `AF_INET` or `AF_INET6` then the buffer is
    /// converted into `SocketAddr`, otherwise the function returns None.
    /// 
    fn into(self) -> Option<SocketAddr>
    {
        unsafe { match self.sa6.sin6_family as i32 {
                libc::AF_INET   => Some(SocketAddr::V4(*(self.as_ptr() as *const _))),
                libc::AF_INET6  => Some(SocketAddr::V6(*(self.as_ptr() as *const _))),
                _ => None
        }}
    }
}

impl From<SocketAddr> for RawSocketAddr
{
    fn from(addr: SocketAddr) -> Self
    {
        Self::from(Some(&addr))
    }
}

impl std::fmt::Debug for RawSocketAddr
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        self.into_addr().fmt(fmt)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddrV6;
    use super::*;

    fn check_as_mut(raw: &mut RawSocketAddr)
    {
        let ptr = raw as *mut _ as usize;
        let buf = raw.as_mut();
        assert_eq!(buf.as_mut_ptr(), ptr as *mut _);
        assert_eq!(buf.len(), std::mem::size_of::<libc::sockaddr_in6>());
    }

    #[test]
    fn ptr_and_capacity() {
        let mut raw = RawSocketAddr::new();
        assert_eq!(raw.as_ptr(), &raw as *const _ as *const _);
        assert_eq!(raw.as_mut_ptr(), &mut raw as *mut _ as *mut _);
        assert_eq!(raw.capacity(), std::mem::size_of::<libc::sockaddr_in6>());
    }

    #[test]
    fn as_slice() {
        let mut raw = RawSocketAddr::new();
        {
            let sl = raw.as_ref();
            assert_eq!(sl.as_ptr(), &raw as *const _ as *const _);
            assert_eq!(sl.len(), 0);
        }
        {
            let ptr = &mut raw as *mut _ as *mut _;
            let sl = raw.as_mut();
            assert_eq!(sl.as_mut_ptr(), ptr);
            assert_eq!(sl.len(), std::mem::size_of::<libc::sockaddr_in6>());
        }
    }

    #[test]
    fn rawsocketaddr_ipv4()
    {
        let addr : SocketAddr = "12.34.56.78:4242".parse().unwrap();
        unsafe {
            let sa = libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_addr: *(&[12u8,34,56,78] as *const _ as *const libc::in_addr),
                sin_port: 4242u16.to_be(),
                sin_zero: std::mem::zeroed(),
            };
            let mut raw = RawSocketAddr::from_raw_parts(&sa as *const _ as *const u8,
                                                    std::mem::size_of_val(&sa));
            assert_eq!(raw.len(),       std::mem::size_of::<libc::sockaddr_in>());
            assert_eq!(raw.capacity(),  std::mem::size_of::<libc::sockaddr_in6>());
            assert_eq!(raw.into_addr(), Some(addr));
            assert_eq!(RawSocketAddr::from(Some(&addr)).into_addr(), Some(addr));
            {
                let buf = raw.as_ref();
                assert_eq!(buf.as_ptr(), &raw as *const _ as *const _);
                assert_eq!(buf.len(), std::mem::size_of_val(&sa));
            } 
            check_as_mut(&mut raw);
        }
    }

    #[test]
    fn rawsocketaddr_ipv6()
    {
        let ip = [7u8,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22];
        let addr = SocketAddr::V6(SocketAddrV6::new(ip.into(), 4242,
                        0x11223344, 0x55667788));
        unsafe {
            let sa = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as u16,
                sin6_addr: *(&ip as *const _ as *const libc::in6_addr),
                sin6_port: 4242u16.to_be(),
                sin6_flowinfo: 0x11223344,
                sin6_scope_id: 0x55667788,
            };
            let mut raw = RawSocketAddr::from_raw_parts(&sa as *const _ as *const u8,
                                                    std::mem::size_of_val(&sa));
            assert_eq!(raw.len(),       std::mem::size_of::<libc::sockaddr_in6>());
            assert_eq!(raw.capacity(),  std::mem::size_of::<libc::sockaddr_in6>());
            assert_eq!(raw.into_addr(), Some(addr));
            assert_eq!(RawSocketAddr::from(Some(&addr)).into_addr(), Some(addr));
            {
                let buf = raw.as_ref();
                assert_eq!(buf.as_ptr(), &raw as *const _ as *const _);
                assert_eq!(buf.len(), std::mem::size_of_val(&sa));
            }
            check_as_mut(&mut raw);
        }
    }

    #[test]
    fn rawsocketaddr_other()
    {
        fn check(raw: &mut RawSocketAddr) {
            assert_eq!(raw.into_addr(), None);
            {
                let buf = raw.as_ref();
                assert_eq!(buf.len(), 0);
                assert_eq!(raw.len(), 0);
                assert_eq!(raw.capacity(), std::mem::size_of::<libc::sockaddr_in6>());
            }
            check_as_mut(raw);
        };

        check(&mut RawSocketAddr::new());
        check(&mut RawSocketAddr::from(None));

        unsafe {
            check(&mut RawSocketAddr::from_raw_parts([0xde,0xad,0xbe,0xef].as_ptr(), 4));
        }
    }
}

