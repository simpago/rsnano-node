mod local_vote_history;
mod vote;
mod vote_spacing;

pub(crate) use local_vote_history::*;
pub(crate) use vote::*;
pub use vote_spacing::VoteSpacing;

use crate::core::Uniquer;
use std::sync::RwLock;

pub(crate) type VoteUniquer = Uniquer<RwLock<Vote>>;

mod election_status;
mod prioritization;
mod recently_cemented_cache;

pub use election_status::{ElectionStatus, ElectionStatusType};
pub use prioritization::{Prioritization, ValueType};
pub use recently_cemented_cache::RecentlyCementedCache;
