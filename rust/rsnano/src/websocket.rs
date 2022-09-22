use anyhow::Result;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::utils::{create_property_tree, PropertyTreeWriter};

#[derive(Clone, Copy, FromPrimitive)]
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

pub struct Message {
    pub topic: Topic,
    pub contents: Box<dyn PropertyTreeWriter>,
}

pub struct MessageBuilder {}

impl MessageBuilder {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn bootstrap_started(id: &str, mode: &str) -> Result<Message> {
        let mut message = Self::new_message()?;
        // Bootstrap information
        let mut bootstrap = create_property_tree();
        bootstrap.put_string("reason", "started")?;
        bootstrap.put_string("id", id)?;
        bootstrap.put_string("mode", mode)?;
        message.contents.add_child("message", bootstrap.as_ref());
        Ok(message)
    }

    pub(crate) fn bootstrap_exited(
        id: &str,
        mode: &str,
        duration: Duration,
        total_blocks: u64,
    ) -> Result<Message> {
        let mut message = Self::new_message()?;
        let mut bootstrap = create_property_tree();
        bootstrap.put_string("reason", "exited")?;
        bootstrap.put_string("id", id)?;
        bootstrap.put_string("mode", mode)?;
        bootstrap.put_u64("total_blocks", total_blocks)?;
        bootstrap.put_u64("duration", duration.as_secs())?;
        message.contents.add_child("message", bootstrap.as_ref());

        Ok(message)
    }

    pub(crate) fn set_common_fields(message: &mut Message) -> Result<()> {
        message.contents.add("topic", from_topic(message.topic))?;
        message.contents.add(
            "time",
            &SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string(),
        )?;
        Ok(())
    }

    fn new_message() -> Result<Message, anyhow::Error> {
        let mut message = Message {
            topic: Topic::Bootstrap,
            contents: create_property_tree(),
        };
        Self::set_common_fields(&mut message)?;
        Ok(message)
    }
}

pub(crate) fn from_topic(topic: Topic) -> &'static str {
    match topic {
        Topic::Ack => "ack",
        Topic::Confirmation => "confirmation",
        Topic::StartedElection => "started_election",
        Topic::StoppedElection => "stopped_election",
        Topic::Vote => "vote",
        Topic::Work => "work",
        Topic::Bootstrap => "bootstrap",
        Topic::Telemetry => "telemetry",
        Topic::NewUnconfirmedBlock => "new_unconfirmed_block",
        _ => "invalid",
    }
}

pub(crate) fn to_topic(topic: impl AsRef<str>) -> Topic {
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

pub trait Listener {
    fn broadcast(&self, message: &Message) -> Result<()>;
}

pub(crate) struct NullListener {}

impl NullListener {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Listener for NullListener {
    fn broadcast(&self, _message: &Message) -> Result<()> {
        Ok(())
    }
}
