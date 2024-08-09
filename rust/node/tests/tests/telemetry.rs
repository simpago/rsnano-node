use rsnano_messages::{Message, TelemetryAck};
use rsnano_node::stats::{DetailType, Direction, StatType};
use std::time::Duration;

use super::helpers::{assert_never, assert_timely, make_fake_channel, System};

#[test]
fn invalid_signature() {
    let mut system = System::new();
    let node = system.make_node();

    let mut telemetry = node.telemetry.local_telemetry();
    telemetry.block_count = 9999; // Change data so signature is no longer valid
    let node_id = telemetry.node_id;
    let message = Message::TelemetryAck(TelemetryAck(Some(telemetry)));

    let channel = make_fake_channel(&node);
    channel.set_node_id(node_id);
    node.inbound_message_queue.put(message, channel);

    assert_timely(Duration::from_secs(5), || {
        node.stats.count(
            StatType::Telemetry,
            DetailType::InvalidSignature,
            Direction::In,
        ) > 0
    });
    assert_never(Duration::from_secs(1), || {
        node.stats
            .count(StatType::Telemetry, DetailType::Process, Direction::In)
            > 0
    });
}
