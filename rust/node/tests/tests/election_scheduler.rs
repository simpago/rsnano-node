use super::helpers::System;
use rsnano_core::Amount;
use rsnano_node::consensus::{Bucket, PriorityBucketConfig};

mod bucket {
    use std::sync::Arc;

    use rsnano_core::{BlockEnum, StateBlock};

    use super::*;

    #[test]
    fn construction() {
        let mut system = System::new();
        let node = system.make_node();

        let bucket = Bucket::new(
            Amount::nano(1000),
            PriorityBucketConfig::default(),
            node.active.clone(),
            node.stats.clone(),
        );

        assert_eq!(bucket.can_accept(Amount::nano(999)), false);
        assert_eq!(bucket.can_accept(Amount::nano(1000)), true);
        assert_eq!(bucket.can_accept(Amount::nano(1001)), true);
        assert_eq!(bucket.len(), 0);
    }

    #[test]
    fn insert_one() {
        let mut system = System::new();
        let node = system.make_node();

        let bucket = Bucket::new(
            Amount::zero(),
            PriorityBucketConfig::default(),
            node.active.clone(),
            node.stats.clone(),
        );

        assert!(bucket.push(1000, Arc::new(BlockEnum::new_test_instance())));
        assert_eq!(bucket.len(), 1);
    }

    #[test]
    fn insert_duplicate() {
        let mut system = System::new();
        let node = system.make_node();

        let bucket = Bucket::new(
            Amount::zero(),
            PriorityBucketConfig::default(),
            node.active.clone(),
            node.stats.clone(),
        );

        let block = Arc::new(BlockEnum::new_test_instance());
        assert_eq!(bucket.push(1000, block.clone()), true);
        assert_eq!(bucket.push(1000, block), false);
    }

    #[test]
    fn insert_many() {
        let mut system = System::new();
        let node = system.make_node();

        let bucket = Bucket::new(
            Amount::zero(),
            PriorityBucketConfig::default(),
            node.active.clone(),
            node.stats.clone(),
        );

        let block0 = Arc::new(BlockEnum::new_test_instance_with_key(1));
        let block1 = Arc::new(BlockEnum::new_test_instance_with_key(2));
        let block2 = Arc::new(BlockEnum::new_test_instance_with_key(3));
        let block3 = Arc::new(BlockEnum::new_test_instance_with_key(3));
        assert!(bucket.push(2000, block0.clone()));
        assert!(bucket.push(1001, block1.clone()));
        assert!(bucket.push(1000, block2.clone()));
        assert!(bucket.push(900, block3.clone()));

        assert_eq!(bucket.len(), 4);
        let blocks = bucket.blocks();
        assert_eq!(blocks.len(), 4);
        // Ensure correct order
        assert_eq!(blocks[0], block3);
        assert_eq!(blocks[1], block2);
        assert_eq!(blocks[2], block1);
        assert_eq!(blocks[3], block0);
    }

    #[test]
    fn max_blocks() {
        let mut system = System::new();
        let node = system.make_node();

        let config = PriorityBucketConfig {
            max_blocks: 2,
            ..Default::default()
        };
        let bucket = Bucket::new(
            Amount::zero(),
            config,
            node.active.clone(),
            node.stats.clone(),
        );

        let block0 = Arc::new(BlockEnum::new_test_instance_with_key(1));
        let block1 = Arc::new(BlockEnum::new_test_instance_with_key(2));
        let block2 = Arc::new(BlockEnum::new_test_instance_with_key(3));
        let block3 = Arc::new(BlockEnum::new_test_instance_with_key(3));

        assert_eq!(bucket.push(2000, block0.clone()), true);
        assert_eq!(bucket.push(900, block1.clone()), true);
        assert_eq!(bucket.push(3000, block2.clone()), false);
        assert_eq!(bucket.push(1001, block3.clone()), true); // Evicts 2000
        assert_eq!(bucket.push(1000, block0.clone()), true); // Evicts 1001

        assert_eq!(bucket.len(), 2);
        let blocks = bucket.blocks();
        // Ensure correct order
        assert_eq!(blocks[0], block1);
        assert_eq!(blocks[1], block0);
    }
}
