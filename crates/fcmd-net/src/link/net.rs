//! The local network, as the Link counts it (SPEC 04 §9.3): a packet or a connection from
//! anywhere else is dropped unread, and none is made to anywhere else.

use std::net::IpAddr;

/// RFC 1918, link-local, IPv6 unique-local and link-local, and loopback (which the tests
/// use).
pub fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_loopback(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_local(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            v6.is_loopback()
                || (first & 0xfe00) == 0xfc00 // fc00::/7, unique local
                || (first & 0xffc0) == 0xfe80 // fe80::/10, link local
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_local_network_and_nothing_else() {
        for ok in [
            "192.168.1.10",
            "10.0.0.1",
            "172.16.5.5",
            "169.254.1.1",
            "127.0.0.1",
            "::1",
            "fd12::1",
            "fe80::1",
            "::ffff:192.168.0.2",
        ] {
            assert!(is_local(ok.parse().unwrap()), "{ok}");
        }
        for no in [
            "8.8.8.8",
            "172.32.0.1",
            "1.1.1.1",
            "2001:db8::1",
            "::ffff:8.8.8.8",
        ] {
            assert!(!is_local(no.parse().unwrap()), "{no}");
        }
    }
}
