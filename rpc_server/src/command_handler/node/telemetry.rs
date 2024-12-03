use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_rpc_messages::{TelemetryArgs, TelemetryDto, TelemetryResponse};
use std::net::SocketAddrV6;

impl RpcCommandHandler {
    pub(crate) fn telemetry(&self, args: TelemetryArgs) -> anyhow::Result<TelemetryResponse> {
        let mut responses = Vec::new();
        if args.address.is_some() || args.port.is_some() {
            // Check both are specified
            let Some(address) = args.address else {
                bail!("Both port and address required");
            };
            let Some(port) = args.port else {
                bail!("Both port and address required");
            };

            let endpoint = SocketAddrV6::new(address, port.into(), 0, 0);

            if address.is_loopback() && port == self.node.tcp_listener.local_address().port().into()
            {
                // Requesting telemetry metrics locally
                let data = self.node.telemetry.local_telemetry();
                responses.push(TelemetryDto::from(data));
                return Ok(TelemetryResponse { metrics: responses });
            } else {
                let Some(telemetry) = self.node.telemetry.get_telemetry(&endpoint) else {
                    bail!("Peer not found");
                };

                responses.push(TelemetryDto::from(telemetry));
            }
        } else {
            // By default, local telemetry metrics are returned,
            // setting "raw" to true returns metrics from all nodes requested.
            let output_raw = args.raw.unwrap_or_default().inner();
            let all_telemetries = self.node.telemetry.get_all_telemetries();
            if output_raw {
                for (addr, data) in all_telemetries {
                    let mut metric = TelemetryDto::from(data);
                    metric.address = Some(addr.ip().clone());
                    metric.port = Some(addr.port().into());
                    responses.push(metric);
                }
            } else {
                // Default case without any parameters, requesting telemetry metrics locally
                let data = self.node.telemetry.local_telemetry();
                responses.push(data.into());
            }
        }

        Ok(TelemetryResponse { metrics: responses })
    }
}
