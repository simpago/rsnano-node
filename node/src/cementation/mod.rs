mod confirming_set;

pub use confirming_set::*;
use rsnano_core::SavedBlock;

type BlockCallback = Box<dyn FnMut(&SavedBlock) + Send>;
type BatchCementedCallback = Box<dyn FnMut(&CementedNotification) + Send>;
