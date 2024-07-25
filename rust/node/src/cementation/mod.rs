mod confirming_set;

pub use confirming_set::*;
use rsnano_core::BlockEnum;
use std::sync::Arc;

type BlockCallback = Box<dyn FnMut(&Arc<BlockEnum>) + Send>;
type BatchCementedCallback = Box<dyn FnMut(&CementedNotification) + Send>;
