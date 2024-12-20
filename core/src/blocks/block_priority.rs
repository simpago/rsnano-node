use crate::{utils::UnixTimestamp, Amount, SavedBlock};
use std::cmp::max;

pub fn block_priority(
    block: &SavedBlock,
    previous_block: Option<&SavedBlock>,
) -> (Amount, UnixTimestamp) {
    let previous_balance = previous_block
        .as_ref()
        .map(|b| b.balance())
        .unwrap_or_default();

    // Handle full send case nicely where the balance would otherwise be 0
    let priority_balance = max(
        block.balance(),
        if block.is_send() {
            previous_balance
        } else {
            Amount::zero()
        },
    );

    // Use previous block timestamp as priority timestamp for least recently used
    // prioritization within the same bucket
    // Account info timestamp is not used here because it will get out of sync when
    // rollbacks happen
    let priority_timestamp = previous_block
        .map(|b| b.timestamp())
        .unwrap_or(block.timestamp());

    (priority_balance, priority_timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PrivateKey, SavedBlockLatticeBuilder};

    #[test]
    fn open_block() {
        let mut lattice = SavedBlockLatticeBuilder::new();
        let key = PrivateKey::from(42);
        let send = lattice.genesis().send(&key, 1);
        lattice.advance_time();
        let open = lattice.account(&key).receive(&send);

        let (prio_balance, prio_time) = block_priority(&open, None);

        assert_eq!(prio_balance, open.balance());
        assert_eq!(prio_time, open.timestamp());
    }

    #[test]
    fn receive_block() {
        let mut lattice = SavedBlockLatticeBuilder::new();
        let key = PrivateKey::from(42);
        let send1 = lattice.genesis().send(&key, 1);
        lattice.advance_time();
        let send2 = lattice.genesis().send(&key, 1);
        lattice.advance_time();
        let open = lattice.account(&key).receive(&send1);
        lattice.advance_time();
        let receive = lattice.account(&key).receive(&send2);

        let (prio_balance, prio_time) = block_priority(&receive, Some(&open));

        assert_eq!(prio_balance, receive.balance());
        assert_eq!(prio_time, open.timestamp());
    }
}
