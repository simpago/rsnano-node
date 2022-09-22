use crate::{Link, PublicKey};
use std::collections::HashMap;

/**
 * Tag for which epoch an entry belongs to
 */

#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy, FromPrimitive, Hash, Default)]
pub enum Epoch {
    Invalid = 0,
    #[default]
    Unspecified = 1,
    Epoch0 = 2,
    Epoch1 = 3,
    Epoch2 = 4,
}

impl Epoch {
    pub const EPOCH_BEGIN: Epoch = Epoch::Epoch0;
    pub const MAX: Epoch = Epoch::Epoch2;
}

#[derive(Clone)]
struct EpochInfo {
    pub signer: PublicKey,
    pub link: Link,
}

#[derive(Clone, Default)]
pub struct Epochs {
    epochs: HashMap<Epoch, EpochInfo>,
}

impl Epochs {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, epoch: Epoch, signer: PublicKey, link: Link) {
        self.epochs.insert(epoch, EpochInfo { signer, link });
    }

    /// Returns true if link matches one of the released epoch links.
    /// WARNING: just because a legal block contains an epoch link, it does not mean it is an epoch block.
    /// A legal block containing an epoch link can easily be constructed by sending to an address identical
    /// to one of the epoch links.
    /// Epoch blocks follow the following rules and a block must satisfy them all to be a true epoch block:
    ///     epoch blocks are always state blocks
    ///     epoch blocks never change the balance of an account
    ///     epoch blocks always have a link field that starts with the ascii bytes "epoch v1 block" or "epoch v2 block" (and possibly others in the future)
    ///     epoch blocks never change the representative
    ///     epoch blocks are not signed by the account key, they are signed either by genesis or by special epoch keys
    pub fn is_epoch_link(&self, link: &Link) -> bool {
        self.epochs.values().any(|x| &x.link == link)
    }

    pub fn link(&self, epoch: Epoch) -> Option<&Link> {
        self.epochs.get(&epoch).map(|x| &x.link)
    }

    pub fn signer(&self, epoch: Epoch) -> Option<&PublicKey> {
        self.epochs.get(&epoch).map(|x| &x.signer)
    }

    pub fn epoch(&self, link: &Link) -> Option<Epoch> {
        for (k, v) in &self.epochs {
            if &v.link == link {
                return Some(*k);
            }
        }

        None
    }

    /// Checks that new_epoch is 1 version higher than epoch
    pub fn is_sequential(epoch: Epoch, new_epoch: Epoch) -> bool {
        // Currently assumes that the epoch versions in the enum are sequential.
        let epoch_id = epoch as u8;
        let new_epoch_id = new_epoch as u8;
        epoch_id >= Epoch::Epoch0 as u8 && new_epoch_id == epoch_id + 1
    }
}
