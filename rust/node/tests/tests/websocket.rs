use futures_util::{SinkExt, StreamExt};
use rsnano_core::{Amount, BlockEnum, KeyPair, Networks, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_messages::{Message, Publish};
use rsnano_node::{
    config::{NetworkConstants, NodeConfig},
    websocket::{OutgoingMessageEnvelope, Topic, WebsocketConfig},
};
use std::time::Duration;
use test_helpers::{assert_timely, get_available_port, make_fake_channel, System};
use tokio::{task::spawn_blocking, time::timeout};

/// Tests getting notification of a started election
#[test]
fn started_election() {
    let mut system = System::new();
    let websocket_port = get_available_port();
    let config = NodeConfig {
        websocket_config: WebsocketConfig {
            enabled: true,
            port: websocket_port,
            ..WebsocketConfig::new(&NetworkConstants::default_for(Networks::NanoDevNetwork))
        },
        ..System::default_config()
    };
    let node1 = system.build_node().config(config).finish();
    let channel1 = make_fake_channel(&node1);
    node1.tokio.block_on(async {
        let (mut ws_stream, _) =
            tokio_tungstenite::connect_async(format!("ws://[::1]:{}", websocket_port))
                .await
                .expect("Failed to connect");
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "started_election", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();

        //await ack
        ws_stream.next().await.unwrap().unwrap();

        assert_eq!(
            1,
            node1
                .websocket
                .as_ref()
                .unwrap()
                .subscriber_count(Topic::StartedElection)
        );

        // Create election, causing a websocket message to be emitted
        let key1 = KeyPair::new();
        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::zero(),
            key1.account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));
        let publish1 = Message::Publish(Publish::new_forward(send1.clone()));
        node1
            .inbound_message_queue
            .put(publish1, channel1.info.clone());
        assert_timely(Duration::from_secs(1), || {
            node1.active.election(&send1.qualified_root()).is_some()
        });

        let Ok(response) = timeout(Duration::from_secs(5), ws_stream.next()).await else {
            panic!("timeout");
        };
        let response = response.unwrap().unwrap();
        let response_msg: OutgoingMessageEnvelope =
            serde_json::from_str(response.to_text().unwrap()).unwrap();
        assert_eq!(response_msg.topic, Some(Topic::StartedElection));
    });
}

// Tests getting notification of an erased election
#[test]
fn stopped_election() {
    let mut system = System::new();
    let websocket_port = get_available_port();
    let config = NodeConfig {
        websocket_config: WebsocketConfig {
            enabled: true,
            port: websocket_port,
            ..WebsocketConfig::new(&NetworkConstants::default_for(Networks::NanoDevNetwork))
        },
        ..System::default_config()
    };
    let node1 = system.build_node().config(config).finish();
    let channel1 = make_fake_channel(&node1);
    node1.tokio.block_on(async {
        let (mut ws_stream, _) =
            tokio_tungstenite::connect_async(format!("ws://[::1]:{}", websocket_port))
                .await
                .expect("Failed to connect");
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "stopped_election", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();

        //await ack
        ws_stream.next().await.unwrap().unwrap();

        assert_eq!(
            1,
            node1
                .websocket
                .as_ref()
                .unwrap()
                .subscriber_count(Topic::StoppedElection)
        );

        // Create election, then erase it, causing a websocket message to be emitted
        let key1 = KeyPair::new();
        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::zero(),
            key1.account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));
        let publish1 = Message::Publish(Publish::new_forward(send1.clone()));
        node1
            .inbound_message_queue
            .put(publish1, channel1.info.clone());
        assert_timely(Duration::from_secs(1), || {
            node1.active.election(&send1.qualified_root()).is_some()
        });
        let active = node1.active.clone();
        spawn_blocking(move || active.erase(&send1.qualified_root()))
            .await
            .unwrap();

        let Ok(response) = timeout(Duration::from_secs(5), ws_stream.next()).await else {
            panic!("timeout");
        };
        let response = response.unwrap().unwrap();
        let response_msg: OutgoingMessageEnvelope =
            serde_json::from_str(response.to_text().unwrap()).unwrap();
        assert_eq!(response_msg.topic, Some(Topic::StoppedElection));
    });
}
