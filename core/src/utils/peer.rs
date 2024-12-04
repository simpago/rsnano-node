use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{net::SocketAddrV6, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    pub address: String,
    pub port: u16,
}

impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.address, self.port)
    }
}

impl Peer {
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
        }
    }

    pub fn parse_list(input: &[String], default_port: u16) -> Vec<Peer> {
        input
            .iter()
            .filter_map(|s| parse_peer(s, default_port).ok())
            .collect()
    }
}

fn parse_peer(input: &str, default_port: u16) -> anyhow::Result<Peer> {
    if input.contains(']') {
        // IPV6 with port
        let addr = SocketAddrV6::from_str(input)?;
        Ok(Peer::new(addr.ip().to_string(), addr.port()))
    } else if input.contains("::") {
        // IPV6 without port
        Ok(Peer::new(input.to_owned(), default_port))
    } else if input.contains(':') {
        // hostname/ipv4 with port
        let mut values = input.split(':');
        let host = values.next().unwrap().to_owned();
        let port = values
            .next()
            .ok_or_else(|| anyhow!("no port"))?
            .parse::<u16>()?;
        Ok(Peer::new(host, port))
    } else {
        // just hostname/ipv4
        Ok(Peer::new(input.to_owned(), default_port))
    }
}

impl FromStr for Peer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid format".into());
        }

        let address = parts[0].to_string();
        let port = parts[1]
            .parse::<u16>()
            .map_err(|_| "Invalid port".to_string())?;

        Ok(Peer { address, port })
    }
}

impl Serialize for Peer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Peer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_peer_serialize() {
        let peer = Peer::new("192.168.1.1", 7075);
        let serialized = serde_json::to_string(&peer).unwrap();
        assert_eq!(serialized, "\"192.168.1.1:7075\"");
    }

    #[test]
    fn test_peer_deserialize() {
        let serialized = "\"192.168.1.1:7075\"";
        let peer: Peer = serde_json::from_str(serialized).unwrap();
        assert_eq!(peer, Peer::new("192.168.1.1", 7075));
    }

    #[test]
    fn test_peer_invalid_deserialize() {
        let invalid_inputs = vec![
            "\"invalid\"",
            "\"192.168.1.1\"",
            "\"192.168.1.1:\"",
            "\"192.168.1.1:abc\"",
            "\"192.168.1.1:65536\"",
        ];

        for input in invalid_inputs {
            let result: Result<Peer, _> = serde_json::from_str(input);
            assert!(result.is_err(), "Expected error for input: {}", input);
        }
    }

    #[test]
    fn parse_empty_list() {
        assert!(Peer::parse_list(&[], 42).is_empty());
    }

    #[test]
    fn parse_host_names_only() {
        assert_eq!(
            Peer::parse_list(
                &[
                    "localhost".to_owned(),
                    "127.0.0.1".to_owned(),
                    "my.other.host".to_owned()
                ],
                42
            ),
            [
                Peer::new("localhost", 42),
                Peer::new("127.0.0.1", 42),
                Peer::new("my.other.host", 42)
            ]
        );
    }

    #[test]
    fn parse_ipv6() {
        assert_eq!(parse_peer("::1", 42).unwrap(), Peer::new("::1", 42));
    }

    #[test]
    fn parse_ipv6_with_port() {
        assert_eq!(parse_peer("[::1]:100", 42).unwrap(), Peer::new("::1", 100));
    }

    #[test]
    fn parse_hostname_with_port() {
        assert_eq!(
            parse_peer("my.host.de:100", 42).unwrap(),
            Peer::new("my.host.de", 100)
        );
    }

    #[test]
    fn should_remove_invalid_peers() {
        assert!(
            Peer::parse_list(&["[asdfasfd]:1".to_owned(), "localhost:".to_owned()], 42).is_empty()
        )
    }
}
