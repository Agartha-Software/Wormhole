use std::fmt::{Debug, Display};
use std::net::Ipv4Addr;

/// Represents an IPv4 address with a port.
///
/// This struct stores an IP address and a port number. Use `parse` to
/// create an instance from a string in the format "x.x.x.x:port".
#[derive(Debug, PartialEq)]
pub struct IpP {
    /// The Ipv4 address.
    pub addr: Ipv4Addr,
    /// The port number.
    pub port: u16,
}

impl IpP {
    /// Set the stored port number.
    ///
    /// ```rust,ignore
    /// let mut ip = IpP { addr: "192.168.0.1".parse().unwrap(), port: 80 };
    /// ip.set_port(8080);
    /// assert_eq!(ip.port, 8080);
    /// ```
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    /// Set the last octet of the stored IP address.
    ///This method replace the last octet of `self.addr` with `value`
    ///
    /// # Exemples
    ///
    /// ```rust,ignore
    /// let mut ip = IpP { addr: "192.168.0.1".parse().unwrap(), port: 80 };
    /// ip.set_ip_last(42);
    /// assert_eq!(ip.addr.octets()[3], 42);
    /// ```
    pub fn set_ip_last(&mut self, value: u8) {
        let mut octets = self.addr.octets();
        octets[3] = value;
        self.addr = Ipv4Addr::from(octets);
    }

    /// Return the last octet (fourth byte) of the stored IPv4 address.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let ip = IpP { addr: "10.0.0.42".parse().unwrap(), port: 1234 };
    /// assert_eq!(ip.get_ip_last(), 42);
    /// ```
    pub fn get_ip_last(&self) -> u8 {
        self.addr.octets()[3]
    }
}

impl TryFrom<&String> for IpP {
    type Error = &'static str;

    /// Attempt to convert from a `&String` containing an address in the
    /// `"IP:PORT"` format.
    ///
    /// This simply forwards to the `&str` implementation by calling
    /// `as_str()` on the `String`.
    fn try_from(addr: &String) -> Result<IpP, Self::Error> {
        IpP::try_from(addr.as_str())
    }
}

impl TryFrom<&str> for IpP {
    type Error = &'static str;

    /// Attempt to parse a socket address from a `&str` in the form
    /// `"x.x.x.x:port"`.
    ///
    /// # Errors
    ///
    /// Returns an error string in these cases:
    /// - if the input does not contain exactly one colon `:` separating
    ///   the IP and the port,
    /// - if the IP portion cannot be parsed into an `Ipv4Addr`,
    /// - if the port portion cannot be parsed into a `u16`.
    fn try_from(addr: &str) -> Result<IpP, Self::Error> {
        let split = addr.split(":").collect::<Vec<&str>>();
        if split.len() != 2 {
            Err("IpP: TryFrom: Invalid ip provided (socket addresses must have a single semicolon (:))")
        } else {
            let addr = split[0].parse().ok().ok_or("failed to parse IP")?;
            let port = split[1].parse().ok().ok_or("failed to parse port")?;

            Ok(Self { addr, port })
        }
    }
}

impl Clone for IpP {
    /// Return a shallow copy of `IpP`.
    ///
    /// The clone duplicates the `Ipv4Addr` and the port number. Both fields
    /// are small and `Copy`, so cloning is inexpensive.
    fn clone(&self) -> Self {
        Self {
            addr: self.addr,
            port: self.port,
        }
    }
}

impl Display for IpP {
    /// Format the `IpP` as `"IP:PORT"`.
    ///
    /// This allows usage with formatting macros like `format!` and `println!`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let ip = IpP { addr: "127.0.0.1".parse().unwrap(), port: 80 };
    /// assert_eq!(format!("{}", ip), "127.0.0.1:80");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}
