use core::panic;
use futures_util::{SinkExt, StreamExt};
use rsnano_core::{
    Account, Amount, Block, JsonBlock, KeyPair, SendBlock, StateBlock, Vote, VoteCode,
    DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_messages::{Message, Publish};
use rsnano_node::{
    bootstrap::{BootstrapInitiatorExt, BootstrapStarted},
    config::NodeConfig,
    Node,
};
use rsnano_websocket_server::WebsocketListener;
use rsnano_websocket_server::{
    vote_received, BlockConfirmed, OutgoingMessageEnvelope, TelemetryReceived, Topic, VoteReceived,
};
use std::{net::Ipv6Addr, sync::Arc, thread::sleep, time::Duration};
use test_helpers::{assert_timely, get_available_port, make_fake_channel, System};
use tokio::{net::TcpStream, task::spawn_blocking, time::timeout};
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};

/// Tests getting notification of a started election
#[test]
fn started_election() {
    let mut system = System::new();
    let (node1, websocket) = create_node_with_websocket(&mut system);
    let channel1 = make_fake_channel(&node1);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "started_election", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();

        //await ack
        ws_stream.next().await.unwrap().unwrap();

        assert_eq!(1, websocket.subscriber_count(Topic::StartedElection));

        // Create election, causing a websocket message to be emitted
        let key1 = KeyPair::new();
        let send1 = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::zero(),
            key1.account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(*DEV_GENESIS_HASH),
        ));
        let publish1 = Message::Publish(Publish::new_forward(send1.clone()));
        node1
            .inbound_message_queue
            .put(publish1, channel1.info.clone());
        assert_timely(Duration::from_secs(1), || {
            node1
                .active_elections
                .election(&send1.qualified_root())
                .is_some()
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
    let (node1, websocket) = create_node_with_websocket(&mut system);
    let channel1 = make_fake_channel(&node1);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "stopped_election", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();

        //await ack
        ws_stream.next().await.unwrap().unwrap();

        assert_eq!(1, websocket.subscriber_count(Topic::StoppedElection));

        // Create election, then erase it, causing a websocket message to be emitted
        let key1 = KeyPair::new();
        let send1 = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::zero(),
            key1.account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(*DEV_GENESIS_HASH),
        ));
        let publish1 = Message::Publish(Publish::new_forward(send1.clone()));
        node1
            .inbound_message_queue
            .put(publish1, channel1.info.clone());
        assert_timely(Duration::from_secs(1), || {
            node1
                .active_elections
                .election(&send1.qualified_root())
                .is_some()
        });
        let active = node1.active_elections.clone();
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

#[test]
// Tests clients subscribing multiple times or unsubscribing without a subscription
fn subscription_edge() {
    let mut system = System::new();
    let (node1, websocket) = create_node_with_websocket(&mut system);
    assert_eq!(websocket.subscriber_count(Topic::Confirmation), 0);

    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();
        assert_eq!(websocket.subscriber_count(Topic::Confirmation), 1);
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();
        assert_eq!(websocket.subscriber_count(Topic::Confirmation), 1);
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "unsubscribe", "topic": "confirmation", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();
        assert_eq!(websocket.subscriber_count(Topic::Confirmation), 0);
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "unsubscribe", "topic": "confirmation", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();
        assert_eq!(websocket.subscriber_count(Topic::Confirmation), 0);
    });
}

#[test]
// Subscribes to block confirmations, confirms a block and then awaits websocket notification
fn confirmation() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        node1.insert_into_wallet(&DEV_GENESIS_KEY);
        let key = KeyPair::new();
        let mut balance = Amount::MAX;
        let send_amount = node1.online_reps.lock().unwrap().quorum_delta() + Amount::raw(1);
        // Quick-confirm a block, legacy blocks should work without filtering
        let mut previous = node1.latest(&DEV_GENESIS_ACCOUNT);
        balance = balance - send_amount;
        let send = Block::LegacySend(SendBlock::new(
            &previous,
            &key.public_key().as_account(),
            &balance,
            &DEV_GENESIS_KEY.private_key(),
            node1.work_generate_dev(previous),
        ));
        previous = send.hash();
        node1.process_active(send);

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Confirmation));

        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "unsubscribe", "topic": "confirmation", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Quick confirm a state block
        balance = balance - send_amount;
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            balance,
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        node1.process_active(send);

        timeout(Duration::from_secs(1), ws_stream.next())
            .await
            .unwrap_err();
    });
}

// Tests the filtering options of block confirmations
#[test]
fn confirmation_options() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true, "options": {"confirmation_type": "active_quorum", "accounts": ["xrb_invalid"]}}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Confirm a state block for an in-wallet account
        node1.insert_into_wallet(&DEV_GENESIS_KEY);
        let key = KeyPair::new();
        let mut balance = Amount::MAX;
        let send_amount = node1.online_reps.lock().unwrap().quorum_delta() + Amount::raw(1);
        let mut previous = node1.latest(&DEV_GENESIS_ACCOUNT);
        balance = balance - send_amount;
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            balance,
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        previous = send.hash();
        node1.process_active(send);

        timeout(Duration::from_secs(1), ws_stream.next())
            .await
            .unwrap_err();

        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true, "options": {"confirmation_type": "active_quorum", "all_local_accounts": true, "include_election_info": true}}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Quick-confirm another block
        balance = balance - send_amount;
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            balance,
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        previous = send.hash();
        node1.process_active(send);

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Confirmation));
        let message: BlockConfirmed  = serde_json::from_value(response_json.message.unwrap()).unwrap();
        let election_info = message.election_info.unwrap();
        assert!(election_info.blocks.parse::<i32>().unwrap() >= 1);
		// Make sure tally and time are non-zero.
        assert_ne!(election_info.tally, "0");
        assert_ne!(election_info.time, "0");
        assert!(election_info.votes.is_none());

        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true, "options":{"confirmation_type": "active_quorum", "all_local_accounts": true} }"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Confirm a legacy block
        // When filtering options are enabled, legacy blocks are always filtered
        balance = balance - send_amount;
        let send = Block::LegacySend(SendBlock::new(&previous, &key.public_key().as_account(), &balance, &DEV_GENESIS_KEY.private_key(),
                node1.work_generate_dev(previous)));
        node1.process_active(send);
        timeout(Duration::from_secs(1), ws_stream.next())
            .await
            .unwrap_err();
    });
}

#[test]
fn confirmation_options_votes() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true, "options":{"confirmation_type": "active_quorum", "include_election_info_with_votes": true, "include_block": false} }"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Confirm a state block for an in-wallet account
        node1.insert_into_wallet(&DEV_GENESIS_KEY);
        let key = KeyPair::new();
        let balance = Amount::MAX;
        let send_amount = node1.config.online_weight_minimum + Amount::raw(1);
        let previous = *DEV_GENESIS_HASH;
        let balance = balance - send_amount;
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            balance,
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        let send_hash = send.hash();
        node1.process_active(send);

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Confirmation));

        let message: BlockConfirmed  = serde_json::from_value(response_json.message.unwrap()).unwrap();
        let election_info = message.election_info.unwrap();
        let votes = election_info.votes.unwrap();
        assert_eq!(votes.len(), 1);
        let vote = &votes[0];
        assert_eq!(vote.representative, DEV_GENESIS_ACCOUNT.encode_account());
        assert_ne!(vote.timestamp, "0");
        assert_eq!(vote.hash, send_hash.to_string());
        assert_eq!(vote.weight, node1.balance(&DEV_GENESIS_ACCOUNT).to_string_dec());
    });
}

#[test]
fn confirmation_options_sideband() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true, "options":{"confirmation_type": "active_quorum", "include_block": false, "include_sideband_info": true} }"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

	    // Confirm a state block for an in-wallet account
        node1.insert_into_wallet(&DEV_GENESIS_KEY);

        let key = KeyPair::new();
        let balance = Amount::MAX;
        let send_amount = node1.config.online_weight_minimum + Amount::raw(1);
        let previous = *DEV_GENESIS_HASH;
        let balance = balance - send_amount;
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            balance,
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        node1.process_active(send);

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Confirmation));

        let message: BlockConfirmed  = serde_json::from_value(response_json.message.unwrap()).unwrap();
        let sideband = message.sideband.unwrap();
		// Make sure height and local_timestamp are non-zero.
        assert_ne!(sideband.height, "0");
        assert_ne!(sideband.local_timestamp, "0");
    });
}

#[test]
// Tests updating options of block confirmations
fn confirmation_options_update() {
    let mut system = System::new();
    let (node1, websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "confirmation", "ack": true, "options":{} }"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

		// Now update filter with an account and wait for a response
        ws_stream
            .send(tungstenite::Message::Text(
                format!(r#"{{"action": "update", "topic": "confirmation", "ack": true, "options":{{"accounts_add": ["{}"]}} }}"#, DEV_GENESIS_ACCOUNT.encode_account()),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Confirm a block
        node1.insert_into_wallet(&DEV_GENESIS_KEY);
        let key = KeyPair::new();
        let previous = *DEV_GENESIS_HASH;
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::nano(1000),
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        let previous = send.hash();
        node1.process_active(send);

        assert_eq!(websocket.subscriber_count(Topic::Confirmation), 1);

        // receive confirmation event
        ws_stream.next().await.unwrap().unwrap();

		// Update the filter again, removing the account
        ws_stream
            .send(tungstenite::Message::Text(
                format!(r#"{{"action": "update", "topic": "confirmation", "ack": true, "options":{{"accounts_del": ["{}"]}} }}"#, DEV_GENESIS_ACCOUNT.encode_account()),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

	    // Confirm another block
        let send2 = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::nano(2000),
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        node1.process_active(send2);

        timeout(Duration::from_secs(1), ws_stream.next())
            .await
            .unwrap_err();
    });
}

#[test]
// Subscribes to votes, sends a block and awaits websocket notification of a vote arrival
fn vote() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "vote", "ack": true }"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Quick-confirm a block
        node1.insert_into_wallet(&DEV_GENESIS_KEY);
        let key = KeyPair::new();
        let previous = *DEV_GENESIS_HASH;
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::nano(1000),
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        node1.process_active(send);

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Vote));
    });
}

#[test]
// Tests vote subscription options - vote type
fn vote_options_type() {
    let mut system = System::new();
    let (node1, websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "vote", "ack": true, "options": {"include_replays": true, "include_indeterminate": false} }"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

	    // Custom made votes for simplicity
        let vote = Vote::new(&DEV_GENESIS_KEY, 0, 0, vec![*DEV_GENESIS_HASH]);

        spawn_blocking(move ||{
            websocket.broadcast(&vote_received(&vote, VoteCode::Replay));
        }).await.unwrap();


        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        let message: VoteReceived  = serde_json::from_value(response_json.message.unwrap()).unwrap();
        assert_eq!(message.vote_type, "replay");
    });
}

#[test]
// Tests vote subscription options - list of representatives
fn vote_options_representatives() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                format!(r#"{{"action": "subscribe", "topic": "vote", "ack": true, "options": {{"representatives": ["{}"]}} }}"#, DEV_GENESIS_ACCOUNT.encode_account()),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        node1.insert_into_wallet(&DEV_GENESIS_KEY);
	    // Quick-confirm a block
        let key = KeyPair::new();
        let mut previous = *DEV_GENESIS_HASH;
        let send_amount = node1.online_reps.lock().unwrap().quorum_delta() + Amount::raw(1);
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - send_amount,
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        previous = send.hash();
        node1.process_active(send);


        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Vote));

		// A list of invalid representatives is the same as no filter
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "vote", "ack": true, "options": {"representatives": ["xrb_invalid"]} }"#.to_string()
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - send_amount * 2,
            key.public_key().as_account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(previous),
        ));
        node1.process_active(send);

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Vote));
    });
}

#[test]
#[ignore = "Disabled, because distributed work generation was temporarily removed"]
fn work() {}

#[test]
// Test client subscribing to notifications for bootstrap
fn bootstrap() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "bootstrap", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Start bootstrap attempt
        let node_l = node1.clone();
        spawn_blocking(move || {
            node_l.bootstrap_initiator.bootstrap(
                true,
                "123abc".to_owned(),
                u32::MAX,
                Account::zero(),
            );
        })
        .await
        .unwrap();

        assert_timely(Duration::from_secs(5), || {
            node1.bootstrap_initiator.current_legacy_attempt().is_none()
        });

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        // Wait for the bootstrap notification
        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();

        // Check the bootstrap notification message
        let message: BootstrapStarted =
            serde_json::from_value(response_json.message.unwrap()).unwrap();
        assert_eq!(message.id, "123abc");
        assert_eq!(message.reason, "started");
        assert_eq!(message.mode, "legacy");

        // Wait for bootstrap finish
        assert_timely(Duration::from_secs(5), || {
            !node1.bootstrap_initiator.in_progress()
        });
    });
}

#[test]
// Tests sending keepalive
fn ws_keepalive() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "ping"}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();
    });
}

#[test]
// Tests sending telemetry
fn telemetry() {
    let mut system = System::new();
    let (node1, _websocket1) = create_node_with_websocket(&mut system);
    let (node2, websocket2) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "telemetry", "ack": true}"#.to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Check the telemetry notification message
        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };
        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::Telemetry));

        // Check the bootstrap notification message
        let message: TelemetryReceived =
            serde_json::from_value(response_json.message.unwrap()).unwrap();
        assert_eq!(
            message.address,
            node2.tcp_listener.local_address().ip().to_string()
        );
        assert_eq!(
            message.port,
            node2.tcp_listener.local_address().port().to_string()
        );

        // Other node should have no subscribers
        assert_eq!(websocket2.subscriber_count(Topic::Telemetry), 0);
    });
}

#[test]
fn new_unconfirmed_block() {
    let mut system = System::new();
    let (node1, _websocket) = create_node_with_websocket(&mut system);
    node1.runtime.block_on(async {
        let mut ws_stream = connect_websocket(&node1).await;
        ws_stream
            .send(tungstenite::Message::Text(
                r#"{"action": "subscribe", "topic": "new_unconfirmed_block", "ack": true}"#
                    .to_string(),
            ))
            .await
            .unwrap();
        //await ack
        ws_stream.next().await.unwrap().unwrap();

        // Process a new block
        let send = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(1),
            (*DEV_GENESIS_ACCOUNT).into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(*DEV_GENESIS_HASH),
        ));
        node1.process_local(send.clone()).unwrap();

        let tungstenite::Message::Text(response) = ws_stream.next().await.unwrap().unwrap() else {
            panic!("not a text message");
        };

        let response_json: OutgoingMessageEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response_json.topic, Some(Topic::NewUnconfirmedBlock));
        assert_eq!(response_json.hash, Some(send.hash()));

        // Check the response
        let msg = response_json.message.unwrap();
        let block: JsonBlock = serde_json::from_value(msg).unwrap();
        let JsonBlock::State(_state) = block else {
            panic!("not a state block")
        };
    });
}

fn create_node_with_websocket(system: &mut System) -> (Arc<Node>, Arc<WebsocketListener>) {
    let websocket_port = get_available_port();
    let mut config = NodeConfig {
        ..System::default_config()
    };
    config.websocket_config.enabled = true;
    config.websocket_config.port = websocket_port;
    config.websocket_config.address = Ipv6Addr::LOCALHOST.to_string();
    let node = system
        .build_node()
        .config(config)
        .finish_with_websocket_server();
    node
}

async fn connect_websocket(node: &Node) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    sleep(Duration::from_millis(5));
    let (ws_stream, _) = connect_async(format!("ws://[::1]:{}", node.config.websocket_config.port))
        .await
        .expect("Failed to connect");
    ws_stream
}
