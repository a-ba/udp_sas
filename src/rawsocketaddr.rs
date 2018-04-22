
use ::std;
use ::std::net::SocketAddr;
use ::libc;

/// A type for handling conversions between std::net::SocketAddr and libc::{sockaddr_in,sockaddr_in6}
/// 
/// This type contains just a buffer enough big to hold a `libc::sockaddr_in` or
/// `libc::sockaddr_in6` struct.
/// 
/// Its content can be arbitrary written using `.as_mut_ptr()`. Then a call to `.into_addr()` will
/// attempt to convert it into `std::net::SocketAddr`.
/// 
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
    pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> Self
    {
        let mut raw = RawSocketAddr{sa6: std::mem::zeroed()};
        let len = len.min(std::mem::size_of::<Self>());
        let src = std::slice::from_raw_parts(ptr, len);
        let dst = std::slice::from_raw_parts_mut(&mut raw as *mut _ as *mut u8, len);
        dst.copy_from_slice(src);
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
    pub fn into_addr(&self) -> Option<SocketAddr>
    {
        unsafe {
            match self.sa6.sin6_family as i32 {
                libc::AF_INET =>
                    Some(SocketAddr::V4(*(&self.sa6 as *const _ as *const _))),
                libc::AF_INET6 =>
                    Some(SocketAddr::V6(*(&self.sa6 as *const _ as *const _))),
                _ => None
            }
        }
    }

    /// Convert the internal buffer into a byte slicea
    /// 
    /// Note: the actual length of slice depends on the value of `.sa_family` inside the buffer:
    /// 
    /// * `AF_INET` -> the size of `sockaddr_in`
    /// * `AF_INET6` -> the size of `sockaddr_in6`
    /// * *other* -> 0 (and the slice origin will be the NULL pointer)
    /// 
    pub fn as_bytes(&self) -> &[u8]
    {
        let len = match self.sa6.sin6_family as i32 {
            libc::AF_INET  => std::mem::size_of::<libc::sockaddr_in >(),
            libc::AF_INET6 => std::mem::size_of::<libc::sockaddr_in6>(),
            _ => 0
        };
        unsafe {
            std::slice::from_raw_parts(match len {
                0 => std::ptr::null(),
                _ => &self.sa6 as *const _ as *const _,
            }, len)
        }
    }

    /// Convert the internal buffer into a mutable byte slice
    pub fn as_bytes_mut(&mut self) -> &mut[u8]
    {
        unsafe {
            std::slice::from_raw_parts_mut(&mut self.sa6 as *mut _ as *mut _, 
                                           std::mem::size_of_val(&self.sa6))
        }
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

    fn check_bytes_mut(raw: &mut RawSocketAddr)
    {
        let ptr = raw as *mut _ as usize;
        let buf = raw.as_bytes_mut();
        assert_eq!(buf.as_mut_ptr(), ptr as *mut _);
        assert_eq!(buf.len(), std::mem::size_of::<libc::sockaddr_in6>());
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
            assert_eq!(raw.into_addr(), Some(addr));
            assert_eq!(RawSocketAddr::from(Some(&addr)).into_addr(), Some(addr));
            {
                let buf = raw.as_bytes();
                assert_eq!(buf.as_ptr(), &raw as *const _ as *const _);
                assert_eq!(buf.len(), std::mem::size_of_val(&sa));
            } 
            check_bytes_mut(&mut raw);
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
            assert_eq!(raw.into_addr(), Some(addr));
            assert_eq!(RawSocketAddr::from(Some(&addr)).into_addr(), Some(addr));
            {
                let buf = raw.as_bytes();
                assert_eq!(buf.as_ptr(), &raw as *const _ as *const _);
                assert_eq!(buf.len(), std::mem::size_of_val(&sa));
            }
            check_bytes_mut(&mut raw);
        }
    }

    #[test]
    fn rawsocketaddr_other()
    {
        fn check(raw: &mut RawSocketAddr) {
            assert_eq!(raw.into_addr(), None);
            {
                let buf = raw.as_bytes();
                assert_eq!(buf.as_ptr(), std::ptr::null());
                assert_eq!(buf.len(), 0);
            }
            check_bytes_mut(raw);
        };

        check(&mut RawSocketAddr::new());
        check(&mut RawSocketAddr::from(None));

        unsafe {
            check(&mut RawSocketAddr::from_raw_parts([0xde,0xad,0xbe,0xef].as_ptr(), 4));
        }
    }
}

