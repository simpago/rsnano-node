use rsnano_core::{BlockHash, DifficultyV1};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn work_generate() {
    let mut system = System::new();
    let node = system.build_node().finish();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let hash = BlockHash::from_bytes([1; 32]);

    let work_generate_dto = node
        .runtime
        .block_on(async { server.client.work_generate(hash).await.unwrap() });

    assert_eq!(hash, work_generate_dto.hash);

    let work: u64 = work_generate_dto.work.into();
    let result_difficulty = node.network_params.work.difficulty(&hash.into(), work);

    assert_eq!(result_difficulty, work_generate_dto.difficulty.inner());

    let expected_multiplier = DifficultyV1::to_multiplier(
        result_difficulty,
        node.ledger.constants.work.threshold_base(),
    );
    assert!((expected_multiplier - work_generate_dto.multiplier.unwrap().inner()).abs() < 1e-6);
}
