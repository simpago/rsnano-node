use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StatsArgs {
    #[serde(rename = "type")]
    pub stats_type: StatsType,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatsType {
    Counters,
    Objects,
    Samples,
    Database,
}
