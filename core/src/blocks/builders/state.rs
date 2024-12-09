use crate::blocks::state_block::EpochBlockArgs;
use crate::work::WorkPool;
use crate::{work::STUB_WORK_POOL, StateBlock};
use crate::{
    Account, Amount, Block, BlockBase, BlockDetails, BlockHash, BlockSideband, Epoch, Link,
    PrivateKey, PublicKey, SavedBlock, Signature, StateBlockArgs,
};
use anyhow::Result;

pub struct TestStateBlockBuilder {
    account: Option<Account>,
    previous: BlockHash,
    representative: PublicKey,
    balance: Amount,
    link: Link,
    priv_key: PrivateKey,
    work: Option<u64>,
    signature: Option<Signature>,
    previous_balance: Option<Amount>,
}

impl TestStateBlockBuilder {
    pub(super) fn new() -> Self {
        let key = PrivateKey::new();
        Self {
            account: None,
            previous: BlockHash::from(2),
            representative: PublicKey::from(3),
            balance: Amount::from(4),
            link: Link::from(5),
            priv_key: key,
            previous_balance: None,
            work: None,
            signature: None,
        }
    }

    pub fn from(mut self, other: &StateBlock) -> Self {
        self.account = Some(other.account());
        self.previous = other.previous();
        self.representative = other.representative();
        self.balance = other.balance();
        self.link = other.link();
        self.signature = Some(other.signature().clone());
        self.work = Some(other.work());
        self
    }

    pub fn previous_balance(mut self, balance: Amount) -> Self {
        self.previous_balance = Some(balance);
        self
    }

    pub fn account(mut self, account: impl Into<Account>) -> Self {
        self.account = Some(account.into());
        self
    }

    pub fn account_address(self, address: impl AsRef<str>) -> Result<Self> {
        Ok(self.account(Account::decode_account(address)?))
    }

    pub fn previous(mut self, previous: impl Into<BlockHash>) -> Self {
        self.previous = previous.into();
        self
    }

    pub fn previous_hex(self, previous: impl AsRef<str>) -> Result<Self> {
        Ok(self.previous(BlockHash::decode_hex(previous)?))
    }

    pub fn representative(mut self, rep: impl Into<PublicKey>) -> Self {
        self.representative = rep.into();
        self
    }

    pub fn representative_address(self, address: impl AsRef<str>) -> Result<Self> {
        Ok(self.representative(Account::decode_account(address)?))
    }

    pub fn balance(mut self, balance: impl Into<Amount>) -> Self {
        self.balance = balance.into();
        self
    }

    pub fn balance_dec(self, balance: impl AsRef<str>) -> Result<Self> {
        Ok(self.balance(balance.as_ref().parse::<u128>()?))
    }

    pub fn amount_sent(self, amount: impl Into<Amount>) -> Self {
        let previous_balance = self
            .previous_balance
            .expect("previous balance not specified");
        self.balance(previous_balance - amount.into())
    }

    pub fn amount_received(self, amount: impl Into<Amount>) -> Self {
        let previous_balance = self
            .previous_balance
            .expect("previous balance not specified");
        self.balance(previous_balance + amount.into())
    }

    pub fn link(mut self, link: impl Into<Link>) -> Self {
        self.link = link.into();
        self
    }

    pub fn link_hex(self, link: impl AsRef<str>) -> Result<Self> {
        Ok(self.link(Link::decode_hex(link)?))
    }

    pub fn key(mut self, key: &PrivateKey) -> Self {
        self.signature = None;
        self.priv_key = key.clone();
        self
    }

    pub fn signature(mut self, signature: Signature) -> Self {
        self.signature = Some(signature);
        self
    }

    pub fn sign_zero(self) -> Self {
        self.signature(Signature::new())
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }

    pub fn zero(mut self) -> Self {
        self.account = Some(Account::zero());
        self.previous = BlockHash::zero();
        self.representative = PublicKey::zero();
        self.balance = Amount::zero();
        self.link = Link::zero();
        self.signature = None;
        self.work = Some(0);
        self
    }

    pub fn build(self) -> Block {
        let account = self.account.unwrap_or_else(|| self.priv_key.account());
        let work = self.work.unwrap_or_else(|| {
            let root = if self.previous.is_zero() {
                account.into()
            } else {
                self.previous.into()
            };
            STUB_WORK_POOL.generate_dev2(root).unwrap()
        });

        let mut block: Block = match self.account {
            Some(account) => {
                // Misuse the epoch block constructor, so that we can create the block
                // for the given account
                EpochBlockArgs {
                    account,
                    previous: self.previous,
                    representative: self.representative,
                    balance: self.balance,
                    link: self.link,
                    epoch_signer: &self.priv_key,
                    work,
                }
                .into()
            }
            None => StateBlockArgs {
                key: &self.priv_key,
                previous: self.previous,
                representative: self.representative,
                balance: self.balance,
                link: self.link,
                work,
            }
            .into(),
        };

        if let Some(signature) = self.signature {
            block.set_signature(signature);
        }

        block
    }

    pub fn build_saved(self) -> SavedBlock {
        let block = self.build();

        let details = BlockDetails::new(Epoch::Epoch0, true, false, false);
        let sideband = BlockSideband::new(
            block.account_field().unwrap(),
            BlockHash::zero(),
            block.balance_field().unwrap(),
            5,
            6,
            details,
            Epoch::Epoch0,
        );
        SavedBlock::new(block, sideband)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockBase, TestBlockBuilder};

    #[test]
    fn state_block() {
        let Block::State(block1) = TestBlockBuilder::state()
            .account(3)
            .previous(1)
            .representative(6)
            .balance(2)
            .link(4)
            .work(5)
            .build()
        else {
            panic!("not a state block")
        };

        assert_eq!(block1.account(), Account::from(3));
        assert_eq!(block1.previous(), BlockHash::from(1));
        assert_eq!(block1.representative(), Account::from(6).into());
        assert_eq!(block1.balance(), Amount::raw(2));
        assert_eq!(block1.link(), Link::from(4));
    }

    #[test]
    fn copy_state_block() -> anyhow::Result<()> {
        let block = TestBlockBuilder::state()
            .account_address("xrb_15nhh1kzw3x8ohez6s75wy3jr6dqgq65oaede1fzk5hqxk4j8ehz7iqtb3to")?
            .previous_hex("FEFBCE274E75148AB31FF63EFB3082EF1126BF72BF3FA9C76A97FD5A9F0EBEC5")?
            .balance_dec("2251569974100400000000000000000000")?
            .representative_address(
                "xrb_1stofnrxuz3cai7ze75o174bpm7scwj9jn3nxsn8ntzg784jf1gzn1jjdkou",
            )?
            .link_hex("E16DD58C1EFA8B521545B0A74375AA994D9FC43828A4266D75ECF57F07A7EE86")?
            .build();

        assert_eq!(
            block.hash().to_string(),
            "2D243F8F92CDD0AD94A1D456A6B15F3BE7A6FCBD98D4C5831D06D15C818CD81F"
        );

        let Block::State(b) = &block else {
            panic!("not a state block")
        };
        let block2 = TestBlockBuilder::state().from(&b).build();
        assert_eq!(
            block2.hash().to_string(),
            "2D243F8F92CDD0AD94A1D456A6B15F3BE7A6FCBD98D4C5831D06D15C818CD81F"
        );

        let block3 = TestBlockBuilder::state()
            .from(&b)
            .sign_zero()
            .work(0)
            .build();
        assert_eq!(
            block3.hash().to_string(),
            "2D243F8F92CDD0AD94A1D456A6B15F3BE7A6FCBD98D4C5831D06D15C818CD81F"
        );
        Ok(())
    }

    #[test]
    /// Make sure manually- and builder constructed all-zero blocks have equal hashes, and check signature.
    fn zeroed_state_block() {
        let key = PrivateKey::from(42);
        let zero_block_manual = TestBlockBuilder::state()
            .account(0)
            .previous(0)
            .representative(0)
            .balance(0)
            .link(0)
            .key(&key)
            .work(0)
            .build();

        let zero_block_build = TestBlockBuilder::state().zero().key(&key).build();
        assert_eq!(zero_block_manual.hash(), zero_block_build.hash());
        key.public_key()
            .verify(
                zero_block_build.hash().as_bytes(),
                zero_block_build.signature(),
            )
            .unwrap();
    }

    #[test]
    fn state_block_from_live_network() -> Result<()> {
        // Test against a random hash from the live network
        let block = TestBlockBuilder::state()
            .account_address("xrb_15nhh1kzw3x8ohez6s75wy3jr6dqgq65oaede1fzk5hqxk4j8ehz7iqtb3to")?
            .previous_hex("FEFBCE274E75148AB31FF63EFB3082EF1126BF72BF3FA9C76A97FD5A9F0EBEC5")?
            .balance_dec("2251569974100400000000000000000000")?
            .representative_address(
                "xrb_1stofnrxuz3cai7ze75o174bpm7scwj9jn3nxsn8ntzg784jf1gzn1jjdkou",
            )?
            .link_hex("E16DD58C1EFA8B521545B0A74375AA994D9FC43828A4266D75ECF57F07A7EE86")?
            .build();
        assert_eq!(
            block.hash().to_string(),
            "2D243F8F92CDD0AD94A1D456A6B15F3BE7A6FCBD98D4C5831D06D15C818CD81F"
        );
        assert!(block.source_field().is_none());
        assert!(block.destination_field().is_none());
        assert_eq!(
            block.link_field().unwrap().encode_hex(),
            "E16DD58C1EFA8B521545B0A74375AA994D9FC43828A4266D75ECF57F07A7EE86"
        );
        Ok(())
    }

    #[test]
    fn state_equality() {
        let key1 = PrivateKey::new();
        let block1: Block = StateBlockArgs {
            key: &key1,
            previous: 1.into(),
            representative: 3.into(),
            balance: 2.into(),
            link: 4.into(),
            work: 5,
        }
        .into();

        let block2 = TestBlockBuilder::state()
            .account(key1.public_key())
            .previous(1)
            .representative(3)
            .balance(2)
            .link(4)
            .key(&key1)
            .work(5)
            .build();

        assert_eq!(block1.hash(), block2.hash());
        assert_eq!(block1.work(), block2.work());
    }
}
