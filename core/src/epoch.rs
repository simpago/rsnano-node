use crate::{Account, Block, Link, PublicKey};
use num_traits::FromPrimitive;
use std::collections::HashMap;

/**
 * Tag for which epoch an entry belongs to
 */

#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy, FromPrimitive, Hash, Default, PartialOrd, Ord)]
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
    pub fn epoch_number(&self) -> u8 {
        match self {
            Epoch::Epoch1 => 1,
            Epoch::Epoch2 => 2,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct EpochInfo {
    pub signer: PublicKey,
    pub link: Link,
}

#[derive(Clone, Default, Debug, PartialEq)]
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

    pub fn validate_epoch_signature(&self, block: &Block) -> anyhow::Result<()> {
        let epoch_signer: PublicKey = self
            .epoch_signer(&block.link_field().unwrap_or_default())
            .ok_or_else(|| anyhow!("not an epoch link!"))?
            .into();

        epoch_signer.verify(block.hash().as_bytes(), block.block_signature())
    }

    pub fn epoch_signer(&self, link: &Link) -> Option<Account> {
        self.signer(self.epoch(link)?).map(|i| i.into())
    }
}

// Epoch is bit packed in BlockDetails. That's why it's max is limited to 4 bits
const_assert!((Epoch::MAX as u8) < (1 << 5));

impl TryFrom<u8> for Epoch {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        FromPrimitive::from_u8(value).ok_or_else(|| anyhow!("invalid epoch value"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_sequential() {
        assert!(Epochs::is_sequential(Epoch::Epoch0, Epoch::Epoch1));
        assert!(Epochs::is_sequential(Epoch::Epoch1, Epoch::Epoch2));

        assert_eq!(Epochs::is_sequential(Epoch::Epoch0, Epoch::Epoch2), false);
        assert_eq!(Epochs::is_sequential(Epoch::Epoch0, Epoch::Invalid), false);
        assert_eq!(
            Epochs::is_sequential(Epoch::Unspecified, Epoch::Epoch1),
            false
        );
        assert_eq!(Epochs::is_sequential(Epoch::Epoch1, Epoch::Epoch0), false);
        assert_eq!(Epochs::is_sequential(Epoch::Epoch2, Epoch::Epoch0), false);
        assert_eq!(Epochs::is_sequential(Epoch::Epoch2, Epoch::Epoch2), false);
    }

    #[test]
    fn epoch_link_empty() {
        let epochs = Epochs::new();
        let link = Link::from(42);
        assert_eq!(epochs.is_epoch_link(&link), false);
        assert_eq!(epochs.signer(Epoch::Epoch1), None);
        assert_eq!(epochs.epoch(&link), None);
        assert_eq!(epochs.link(Epoch::Epoch1), None);
    }

    #[test]
    fn epoch_link_for_epoch1() {
        let mut epochs = Epochs::new();
        let link1 = Link::from(42);
        let link2 = Link::from(43);
        let epoch_key = PublicKey::from(100);

        epochs.add(Epoch::Epoch1, epoch_key, link1);

        assert_eq!(epochs.is_epoch_link(&link1), true);
        assert_eq!(epochs.is_epoch_link(&link2), false);
        assert_eq!(epochs.signer(Epoch::Epoch1), Some(&epoch_key));
        assert_eq!(epochs.link(Epoch::Epoch1), Some(&link1));
        assert_eq!(epochs.epoch(&link1), Some(Epoch::Epoch1));
    }

    #[test]
    fn epoch_link_for_epoch2() {
        let mut epochs = Epochs::new();
        let link1 = Link::from(42);
        let link2 = Link::from(43);
        let epoch_key1 = PublicKey::from(100);
        let epoch_key2 = PublicKey::from(200);

        epochs.add(Epoch::Epoch1, epoch_key1, link1);
        epochs.add(Epoch::Epoch2, epoch_key2, link2);

        assert_eq!(epochs.is_epoch_link(&link2), true);
        assert_eq!(epochs.signer(Epoch::Epoch2), Some(&epoch_key2));
        assert_eq!(epochs.link(Epoch::Epoch2), Some(&link2));
        assert_eq!(epochs.epoch(&link2), Some(Epoch::Epoch2));
    }
}
