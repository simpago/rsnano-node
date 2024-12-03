#[macro_use]
extern crate num_derive;

use rsnano_core::{
    utils::{milliseconds_since_epoch, PropertyTree, SerdePropertyTree},
    work::WorkThresholds,
    BlockHash, DifficultyV1, SavedBlock, WorkVersion,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Debug, hash::Hash, time::Duration};

#[derive(Clone, Copy, FromPrimitive, PartialEq, Eq, Hash, Serialize, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Invalid = 0,
    /// Acknowledgement of prior incoming message
    Ack,
    /// A confirmation message
    Confirmation,
    StartedElection,
    /// Stopped election message (dropped elections due to bounding or block lost the elections)
    StoppedElection,
    /// A vote message
    Vote,
    /// Work generation message
    Work,
    /// A bootstrap message
    Bootstrap,
    /// A telemetry message
    Telemetry,
    /// New block arrival message
    NewUnconfirmedBlock,
    /// Auxiliary length, not a valid topic, must be the last enum
    Length,
}

#[derive(Deserialize)]
pub struct IncomingMessage<'a> {
    pub action: Option<&'a str>,
    pub topic: Option<&'a str>,
    #[serde(default)]
    pub ack: bool,
    pub id: Option<&'a str>,
    pub options: Option<Value>,
    #[serde(default)]
    pub accounts_add: Vec<&'a str>,
    #[serde(default)]
    pub accounts_del: Vec<&'a str>,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct OutgoingMessageEnvelope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<Topic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<BlockHash>,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Value>,
}

impl OutgoingMessageEnvelope {
    pub fn new(topic: Topic, message: impl Serialize) -> Self {
        Self {
            id: None,
            ack: None,
            topic: Some(topic),
            hash: None,
            time: milliseconds_since_epoch().to_string(),
            message: Some(serde_json::to_value(message).expect("could not serialize message")),
        }
    }

    pub fn new_ack(id: Option<String>, action: String) -> Self {
        Self {
            id,
            topic: None,
            ack: Some(action),
            hash: None,
            time: milliseconds_since_epoch().to_string(),
            message: None,
        }
    }
}

#[derive(Serialize)]
struct WorkGeneration<'a> {
    success: &'a str,
    reason: &'a str,
    duration: String,
    request: WorkRequest<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<WorkResult>,
    bad_peers: Vec<String>,
}

#[derive(Serialize)]
struct WorkRequest<'a> {
    version: &'a str,
    hash: String,
    difficulty: String,
    multiplier: String,
}

#[derive(Serialize)]
struct WorkResult {
    source: String,
    work: String,
    difficulty: String,
    multiplier: String,
}

pub fn work_generation_message(
    root: &BlockHash,
    work: u64,
    difficulty: u64,
    publish_threshold: u64,
    duration: Duration,
    peer: &str,
    bad_peers: &[String],
    completed: bool,
    cancelled: bool,
) -> OutgoingMessageEnvelope {
    let request_multiplier = DifficultyV1::to_multiplier(difficulty, publish_threshold);
    let request = WorkRequest {
        version: WorkVersion::Work1.as_str(),
        hash: root.to_string(),
        difficulty: format!("{:016x}", difficulty),
        multiplier: format!("{:.10}", request_multiplier),
    };

    let result = if completed {
        let result_difficulty = WorkThresholds::publish_full().difficulty(&root.into(), work);

        let result_multiplier = DifficultyV1::to_multiplier(result_difficulty, publish_threshold);

        Some(WorkResult {
            source: peer.to_string(),
            work: format!("{:016x}", work),
            difficulty: format!("{:016x}", result_difficulty),
            multiplier: format!("{:.10}", result_multiplier),
        })
    } else {
        None
    };

    let bad_peers = bad_peers.iter().cloned().collect();
    OutgoingMessageEnvelope::new(
        Topic::Work,
        WorkGeneration {
            success: if completed { "true" } else { "false" },
            reason: if completed {
                ""
            } else if cancelled {
                "cancelled"
            } else {
                "failure"
            },
            duration: duration.as_millis().to_string(),
            request,
            result,
            bad_peers,
        },
    )
}

pub fn to_topic(topic: impl AsRef<str>) -> Topic {
    match topic.as_ref() {
        "confirmation" => Topic::Confirmation,
        "started_election" => Topic::StartedElection,
        "stopped_election" => Topic::StoppedElection,
        "vote" => Topic::Vote,
        "ack" => Topic::Ack,
        "work" => Topic::Work,
        "bootstrap" => Topic::Bootstrap,
        "telemetry" => Topic::Telemetry,
        "new_unconfirmed_block" => Topic::NewUnconfirmedBlock,
        _ => Topic::Invalid,
    }
}

pub fn new_block_arrived_message(block: &SavedBlock) -> OutgoingMessageEnvelope {
    let mut json_block = SerdePropertyTree::new();
    block.serialize_json(&mut json_block).unwrap();
    let subtype = block.subtype().as_str();
    json_block.put_string("subtype", subtype).unwrap();
    let mut result = OutgoingMessageEnvelope::new(Topic::NewUnconfirmedBlock, json_block.value);
    result.hash = Some(block.hash());
    result
}
