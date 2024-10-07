use crate::RpcCommand;
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;

impl RpcCommand {
    pub fn bootstrap(address: Ipv6Addr, port: u16, id: Option<String>) -> Self {
        Self::Bootstrap(BootstrapArgs::new(address, port, id))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BootstrapArgs {
    pub address: Ipv6Addr,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl BootstrapArgs {
    pub fn new(address: Ipv6Addr, port: u16, id: Option<String>) -> Self {
        Self { address, port, id }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::{from_str, to_string_pretty};
    use std::{net::Ipv6Addr, str::FromStr};

    #[test]
    fn serialize_bootstrap_command_id_none() {
        let address = Ipv6Addr::from_str("::ffff:192.169.0.1").unwrap();

        assert_eq!(
            to_string_pretty(&RpcCommand::bootstrap(address, 1024, None)).unwrap(),
            r#"{
  "action": "bootstrap",
  "address": "::ffff:192.169.0.1",
  "port": 1024
}"#
        )
    }

    #[test]
    fn deserialize_bootstrap_command_none() {
        let address = Ipv6Addr::from_str("::ffff:192.169.0.1").unwrap();
        let cmd = RpcCommand::bootstrap(address, 1024, None);
        let serialized = to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn serialize_bootstrap_command_id_some() {
        let address = Ipv6Addr::from_str("::ffff:192.169.0.1").unwrap();
        assert_eq!(
            to_string_pretty(&RpcCommand::bootstrap(
                address,
                1024,
                Some("id".to_string())
            ))
            .unwrap(),
            r#"{
  "action": "bootstrap",
  "address": "::ffff:192.169.0.1",
  "port": 1024,
  "id": "id"
}"#
        )
    }

    #[test]
    fn deserialize_bootstrap_command_id_some() {
        let address = Ipv6Addr::from_str("::ffff:192.169.0.1").unwrap();
        let cmd = RpcCommand::bootstrap(address, 1024, Some("id".to_string()));
        let serialized = to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized);
    }
}
