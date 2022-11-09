use std::{sync::Arc, thread};

use rsnano::{
    core::{Account, Amount, Block, BlockHash, KeyPair, Link, PublicKey, Signature, StateBlock},
    signatures::{SignatureCheckSet, SignatureChecker},
};

// original test: signature_checker.bulk_single_thread
#[test]
fn bulk_single_thread() {
    let key = KeyPair::new();
    let block = test_state_block(&key);

    let checker = SignatureChecker::new(0);
    let size = 1000;
    let mut hashes = Vec::<BlockHash>::with_capacity(size);
    let mut messages = Vec::<Vec<u8>>::with_capacity(size);
    let mut pub_keys = Vec::<PublicKey>::with_capacity(size);
    let mut signatures = Vec::<Signature>::with_capacity(size);
    let verifications = vec![0; size];
    let mut accounts = Vec::<Account>::with_capacity(size);
    for _ in 0..size {
        let hash = block.hash();
        hashes.push(hash);
        messages.push(hash.as_bytes().to_vec());
        accounts.push(block.account());
        pub_keys.push(block.account().into());
        signatures.push(block.signature.clone())
    }
    let mut check = SignatureCheckSet {
        messages,
        pub_keys,
        signatures,
        verifications,
    };
    checker.verify(&mut check);
    let all_valid = check.verifications.iter().all(|&x| x == 1);
    assert!(all_valid);
}

// original test: signature_checker.many_multi_threaded
#[test]
fn many_multi_threaded() {
    let checker = Arc::new(SignatureChecker::new(4));

    let signature_checker_work_func = move || {
        let key = KeyPair::new();
        let block = test_state_block(&key);

        let block_hash = block.hash();
        let block_account = block.account();
        let block_signature = block.signature;

        let mut invalid_block = test_state_block(&key);
        let mut sig_bytes = *invalid_block.signature.as_bytes();
        sig_bytes[31] ^= 1;
        invalid_block.signature = Signature::from_bytes(sig_bytes);
        let invalid_block_hash = invalid_block.hash();
        let invalid_block_account = invalid_block.account();
        let invalid_block_signature = invalid_block.signature.clone();
        const NUM_CHECK_SIZES: usize = 18;
        const CHECK_SIZES: &'static [usize; NUM_CHECK_SIZES] = &[
            2048, 256, 1024, 1, 4096, 512, 2050, 1024, 8092, 513, 17, 1024, 2047, 255, 513, 2049,
            1025, 1023,
        ];

        // Populate all the signature check sets. The last one in each set is given an incorrect block signature.
        for i in 0..NUM_CHECK_SIZES {
            let check_size = CHECK_SIZES[i];
            assert!(check_size > 0);
            let last_signature_index = check_size - 1;

            let mut messages = vec![block_hash.as_bytes().to_vec(); check_size];
            messages[last_signature_index] = invalid_block_hash.as_bytes().to_vec();

            let mut pub_keys = vec![block_account.into(); check_size];
            pub_keys[last_signature_index] = invalid_block_account.into();

            let mut signatures = Vec::with_capacity(check_size);
            for _ in 0..check_size - 1 {
                signatures.push(block_signature.clone());
            }
            signatures.push(invalid_block_signature.clone());

            let verifications = vec![-1; check_size];
            let mut check_set = SignatureCheckSet {
                messages,
                pub_keys,
                signatures,
                verifications,
            };

            checker.verify(&mut check_set);

            // Confirm all but last are valid
            let all_valid = check_set.verifications[..check_size - 1]
                .iter()
                .all(|&x| x == 1);
            assert!(all_valid);
            assert_eq!(check_set.verifications[last_signature_index], 0);
        }
    };
    let signature_checker_thread1 = thread::spawn(signature_checker_work_func.clone());
    let signature_checker_thread2 = thread::spawn(signature_checker_work_func);
    signature_checker_thread1.join().unwrap();
    signature_checker_thread2.join().unwrap();
}

// original test: signature_checker.boundary_checks
#[test]
fn boundary_checks() {
    // sizes container must be in incrementing order
    let mut sizes = vec![0, 1];
    let mut add_boundary = |boundary: usize| {
        sizes.push(boundary - 1);
        sizes.push(boundary);
        sizes.push(boundary + 1);
    };

    for i in 1..=5 {
        add_boundary(SignatureChecker::BATCH_SIZE * i);
    }

    let checker = SignatureChecker::new(1);
    let max_size = *sizes.last().unwrap();

    let key = KeyPair::new();
    let block = test_state_block(&key);

    let mut last_size = 0;
    let mut check = SignatureCheckSet {
        messages: Vec::with_capacity(max_size),
        pub_keys: Vec::with_capacity(max_size),
        signatures: Vec::with_capacity(max_size),
        verifications: Vec::with_capacity(max_size),
    };

    for size in sizes {
        // The size needed to append to existing containers, saves re-initializing from scratch each iteration
        let extra_size = size - last_size;
        for _ in 0..extra_size {
            check.messages.push(block.hash().as_bytes().to_vec());
            check.pub_keys.push(block.hashables.account.into());
            check.signatures.push(block.signature.clone());
            check.verifications.push(-1);
        }

        checker.verify(&mut check);
        assert!(check.verifications.iter().all(|&x| x == 1));
        last_size = size;
    }
}

fn test_state_block(key: &KeyPair) -> StateBlock {
    let block = StateBlock::new(
        key.public_key().into(),
        BlockHash::zero(),
        key.public_key().into(),
        Amount::zero(),
        Link::zero(),
        &key.private_key(),
        &key.public_key(),
        0,
    )
    .unwrap();
    block
}
