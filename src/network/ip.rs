use std::fmt::{Debug, Display};
use std::net::Ipv4Addr;

// stands for ip and port
#[derive(Debug, PartialEq)]
pub struct IpP {
    pub addr: Ipv4Addr,
    pub port: u16,
}

impl IpP {
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn set_ip_last(&mut self, value: u8) {
        let mut octets = self.addr.octets();
        octets[3] = value;
        self.addr = Ipv4Addr::from(octets);
    }

    pub fn get_ip_last(&self) -> u8 {
        self.addr.octets()[3]
    }
}

impl TryFrom<&String> for IpP {
    type Error = &'static str;
    fn try_from(addr: &String) -> Result<IpP, Self::Error> {
        IpP::try_from(addr.as_str())
    }
}

impl TryFrom<&str> for IpP {
    type Error = &'static str;
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
    fn clone(&self) -> Self {
        Self {
            addr: self.addr,
            port: self.port,
        }
    }
}

impl Display for IpP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}
