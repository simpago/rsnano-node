/// Information on the status of the inactive cache
#[derive(Clone, Default)]
pub struct InactiveCacheStatus {
    pub bootstrap_started: bool,
    pub election_started: bool,
    pub confirmed: bool,
    pub tally: u128,
}