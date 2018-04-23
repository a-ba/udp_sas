[![Build Status](https://travis-ci.org/a-ba/udp_sas.svg?branch=master)](https://travis-ci.org/a-ba/udp_sas)
[![Crates.io](https://img.shields.io/crates/v/udp_sas.svg)](https://crates.io/crates/udp_sas)


This crate provides an extension trait for `std::net::UdpSocket` that supports
source address selection for outgoing UDP datagrams. This is useful for
implementing a UDP server that binds multiple network interfaces.
 
The implementation relies on socket options
[`IP_PKTINFO`](http://man7.org/linux/man-pages/man7/ip.7.html) (for IPv4) and
[`IPV6_RECVPKTINFO`](http://man7.org/linux/man-pages/man7/ipv6.7.html)
(for IPv6).
 
