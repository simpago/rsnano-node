#include <nano/lib/blocks.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/test_common/chains.hpp>
#include <nano/test_common/network.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

using namespace std::chrono_literals;

// Bootstrap can pull one basic block
TEST (bootstrap_processor, process_one)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	node_config.enable_voting = false;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	auto node0 = system.add_node (node_config, node_flags);
	auto wallet_id = node0->wallets.first_wallet_id ();
	(void)node0->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	auto send (node0->wallets.send_action (wallet_id, nano::dev::genesis_key.pub, nano::dev::genesis_key.pub, 100));
	ASSERT_NE (nullptr, send);
	ASSERT_TIMELY (5s, node0->latest (nano::dev::genesis_key.pub) != nano::dev::genesis->hash ());

	node_flags.set_disable_rep_crawler (true);
	node_config.peering_port = system.get_available_port ();
	auto node1 = system.make_disconnected_node (node_config, node_flags);
	ASSERT_NE (node0->latest (nano::dev::genesis_key.pub), node1->latest (nano::dev::genesis_key.pub));
	node1->connect (node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap (node0->network->endpoint ());
	ASSERT_TIMELY_EQ (10s, node1->latest (nano::dev::genesis_key.pub), node0->latest (nano::dev::genesis_key.pub));
}

TEST (bootstrap_processor, process_two)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	auto node0 (system.add_node (config, node_flags));
	auto wallet_id = node0->wallets.first_wallet_id ();
	(void)node0->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	ASSERT_TRUE (node0->wallets.send_action (wallet_id, nano::dev::genesis_key.pub, nano::dev::genesis_key.pub, 50));
	ASSERT_TRUE (node0->wallets.send_action (wallet_id, nano::dev::genesis_key.pub, nano::dev::genesis_key.pub, 50));
	ASSERT_TIMELY_EQ (5s, nano::test::account_info (*node0, nano::dev::genesis_key.pub).block_count (), 3);

	// create a node manually to avoid making automatic network connections
	auto node1 = system.make_disconnected_node ();
	ASSERT_NE (node1->latest (nano::dev::genesis_key.pub), node0->latest (nano::dev::genesis_key.pub)); // nodes should be out of sync here
	node1->connect (node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap (node0->network->endpoint ()); // bootstrap triggered
	ASSERT_TIMELY_EQ (5s, node1->latest (nano::dev::genesis_key.pub), node0->latest (nano::dev::genesis_key.pub)); // nodes should sync up
}

// Bootstrap can pull universal blocks
TEST (bootstrap_processor, process_state)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	auto node0 (system.add_node (config, node_flags));
	nano::state_block_builder builder;

	auto wallet_id = node0->wallets.first_wallet_id ();
	(void)node0->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	auto block1 = builder
				  .account (nano::dev::genesis_key.pub)
				  .previous (node0->latest (nano::dev::genesis_key.pub))
				  .representative (nano::dev::genesis_key.pub)
				  .balance (nano::dev::constants.genesis_amount - 100)
				  .link (nano::dev::genesis_key.pub)
				  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				  .work (0)
				  .build ();
	auto block2 = builder
				  .make_block ()
				  .account (nano::dev::genesis_key.pub)
				  .previous (block1->hash ())
				  .representative (nano::dev::genesis_key.pub)
				  .balance (nano::dev::constants.genesis_amount)
				  .link (block1->hash ())
				  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				  .work (0)
				  .build ();

	node0->work_generate_blocking (*block1);
	node0->work_generate_blocking (*block2);
	ASSERT_EQ (nano::block_status::progress, node0->process (block1));
	ASSERT_EQ (nano::block_status::progress, node0->process (block2));
	ASSERT_TIMELY_EQ (5s, nano::test::account_info (*node0, nano::dev::genesis_key.pub).block_count (), 3);

	auto node1 = system.make_disconnected_node (std::nullopt, node_flags);
	ASSERT_EQ (node0->latest (nano::dev::genesis_key.pub), block2->hash ());
	ASSERT_NE (node1->latest (nano::dev::genesis_key.pub), block2->hash ());
	node1->connect (node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap (node0->network->endpoint ());
	ASSERT_TIMELY_EQ (5s, node1->latest (nano::dev::genesis_key.pub), block2->hash ());
}

TEST (bootstrap_processor, process_new)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	nano::keypair key2;

	auto node1 = system.add_node (config, node_flags);
	config.peering_port = system.get_available_port ();
	auto node2 = system.add_node (config, node_flags);

	auto wallet_id1 = node1->wallets.first_wallet_id ();
	auto wallet_id2 = node2->wallets.first_wallet_id ();
	(void)node1->wallets.insert_adhoc (wallet_id1, nano::dev::genesis_key.prv);
	(void)node2->wallets.insert_adhoc (wallet_id2, key2.prv);

	// send amount raw from genesis to key2, the wallet will autoreceive
	auto amount = node1->config->receive_minimum.number ();
	auto send (node1->wallets.send_action (wallet_id1, nano::dev::genesis_key.pub, key2.pub, amount));
	ASSERT_NE (nullptr, send);
	ASSERT_TIMELY (5s, !node1->balance (key2.pub).is_zero ());

	// wait for the receive block on node2
	std::shared_ptr<nano::block> receive;
	ASSERT_TIMELY (5s, receive = node2->block (node2->latest (key2.pub)));

	// All blocks should be propagated & confirmed
	ASSERT_TIMELY (5s, nano::test::confirmed (*node1, { send, receive }));
	ASSERT_TIMELY (5s, nano::test::confirmed (*node2, { send, receive }));
	ASSERT_TIMELY (5s, node1->active.empty ());
	ASSERT_TIMELY (5s, node2->active.empty ());

	// create a node manually to avoid making automatic network connections
	auto node3 = system.make_disconnected_node ();
	node3->connect (node1->network->endpoint ());
	node3->bootstrap_initiator.bootstrap (node1->network->endpoint ());
	ASSERT_TIMELY_EQ (5s, node3->balance (key2.pub), amount);
	node3->stop ();
}

TEST (bootstrap_processor, pull_diamond)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	auto node0 (system.add_node (config, node_flags));
	nano::keypair key;
	nano::block_builder builder;
	auto send1 = builder
				 .send ()
				 .previous (node0->latest (nano::dev::genesis_key.pub))
				 .destination (key.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (node0->latest (nano::dev::genesis_key.pub)))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node0->process (send1));
	auto open = builder
				.open ()
				.source (send1->hash ())
				.representative (1)
				.account (key.pub)
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();
	ASSERT_EQ (nano::block_status::progress, node0->process (open));
	auto send2 = builder
				 .send ()
				 .previous (open->hash ())
				 .destination (nano::dev::genesis_key.pub)
				 .balance (std::numeric_limits<nano::uint128_t>::max () - 100)
				 .sign (key.prv, key.pub)
				 .work (*system.work.generate (open->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node0->process (send2));
	auto receive = builder
				   .receive ()
				   .previous (send1->hash ())
				   .source (send2->hash ())
				   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				   .work (*system.work.generate (send1->hash ()))
				   .build ();
	ASSERT_EQ (nano::block_status::progress, node0->process (receive));

	auto node1 = system.make_disconnected_node ();
	node1->connect (node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap (node0->network->endpoint ());
	ASSERT_TIMELY_EQ (5s, node1->balance (nano::dev::genesis_key.pub), 100);
}

TEST (bootstrap_processor, push_diamond_pruning)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags0;
	node_flags0.set_disable_ascending_bootstrap (true);
	node_flags0.set_disable_ongoing_bootstrap (true);
	auto node0 (system.add_node (config, node_flags0));
	nano::keypair key;

	config.enable_voting = false; // Remove after allowing pruned voting
	nano::node_flags node_flags;
	node_flags.set_enable_pruning (true);
	config.peering_port = system.get_available_port ();
	auto node1 = system.make_disconnected_node (config, node_flags);

	nano::block_builder builder;

	// send all balance from genesis to key
	auto send1 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (nano::dev::genesis->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send1));

	// receive all balance on key
	auto open = builder
				.open ()
				.source (send1->hash ())
				.representative (1)
				.account (key.pub)
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (open));

	// 1st bootstrap
	node1->connect (node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap (node0->network->endpoint ());
	ASSERT_TIMELY_EQ (5s, node0->balance (key.pub), nano::dev::constants.genesis_amount);
	ASSERT_TIMELY_EQ (5s, node1->balance (key.pub), nano::dev::constants.genesis_amount);

	// Process more blocks & prune old

	// send 100 raw from key to genesis
	auto send2 = builder
				 .send ()
				 .previous (open->hash ())
				 .destination (nano::dev::genesis_key.pub)
				 .balance (std::numeric_limits<nano::uint128_t>::max () - 100)
				 .sign (key.prv, key.pub)
				 .work (*system.work.generate (open->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send2));

	// receive the 100 raw from key on genesis
	auto receive = builder
				   .receive ()
				   .previous (send1->hash ())
				   .source (send2->hash ())
				   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				   .work (*system.work.generate (send1->hash ()))
				   .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (receive));

	{
		auto transaction (node1->store.tx_begin_write ());
		node1->ledger.confirm (*transaction, open->hash ());
		ASSERT_EQ (1, node1->ledger.pruning_action (*transaction, send1->hash (), 2));
		ASSERT_EQ (1, node1->ledger.pruning_action (*transaction, open->hash (), 1));
		ASSERT_TRUE (node1->ledger.any ().block_exists (*transaction, nano::dev::genesis->hash ()));
		ASSERT_FALSE (node1->ledger.any ().block_exists (*transaction, send1->hash ()));
		ASSERT_TRUE (node1->store.pruned ().exists (*transaction, send1->hash ()));
		ASSERT_FALSE (node1->ledger.any ().block_exists (*transaction, open->hash ()));
		ASSERT_TRUE (node1->store.pruned ().exists (*transaction, open->hash ()));
		ASSERT_TRUE (node1->ledger.any ().block_exists (*transaction, send2->hash ()));
		ASSERT_TRUE (node1->ledger.any ().block_exists (*transaction, receive->hash ()));
		ASSERT_EQ (2, node1->ledger.pruned_count ());
		ASSERT_EQ (5, node1->ledger.block_count ());
	}

	// 2nd bootstrap
	node1->connect (node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap (node0->network->endpoint ());
	ASSERT_TIMELY_EQ (5s, node0->balance (nano::dev::genesis_key.pub), 100);
	ASSERT_TIMELY_EQ (5s, node1->balance (nano::dev::genesis_key.pub), 100);
}

TEST (bootstrap_processor, push_one)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto node0 (system.add_node (config));
	nano::keypair key1;
	auto node1 = system.make_disconnected_node ();
	auto wallet_id{ nano::random_wallet_id () };
	node1->wallets.create (wallet_id);
	nano::account account;
	ASSERT_EQ (nano::wallets_error::none, node1->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv, true, account));

	// send 100 raw from genesis to key1
	nano::uint128_t genesis_balance = node1->balance (nano::dev::genesis_key.pub);
	auto send = node1->wallets.send_action (wallet_id, nano::dev::genesis_key.pub, key1.pub, 100);
	ASSERT_NE (nullptr, send);
	ASSERT_TIMELY_EQ (5s, genesis_balance - 100, node1->balance (nano::dev::genesis_key.pub));

	node1->connect (node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap (node0->network->endpoint ());
	ASSERT_TIMELY_EQ (5s, node0->balance (nano::dev::genesis_key.pub), genesis_balance - 100);
}

TEST (bootstrap_processor, lazy_hash_pruning)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	config.enable_voting = false; // Remove after allowing pruned voting
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_enable_pruning (true);
	auto node0 = system.add_node (config, node_flags);

	nano::state_block_builder builder;

	// send Gxrb_ratio raw from genesis to genesis
	auto send1 = builder
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (nano::dev::genesis_key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*node0->work_generate_blocking (nano::dev::genesis->hash ()))
				 .build ();

	// receive send1
	auto receive1 = builder
					.make_block ()
					.account (nano::dev::genesis_key.pub)
					.previous (send1->hash ())
					.representative (nano::dev::genesis_key.pub)
					.balance (nano::dev::constants.genesis_amount)
					.link (send1->hash ())
					.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					.work (*node0->work_generate_blocking (send1->hash ()))
					.build ();

	// change rep of genesis account to be key1
	nano::keypair key1;
	auto change1 = builder
				   .make_block ()
				   .account (nano::dev::genesis_key.pub)
				   .previous (receive1->hash ())
				   .representative (key1.pub)
				   .balance (nano::dev::constants.genesis_amount)
				   .link (0)
				   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				   .work (*node0->work_generate_blocking (receive1->hash ()))
				   .build ();

	// change rep of genesis account to be rep2
	nano::keypair key2;
	auto change2 = builder
				   .make_block ()
				   .account (nano::dev::genesis_key.pub)
				   .previous (change1->hash ())
				   .representative (key2.pub)
				   .balance (nano::dev::constants.genesis_amount)
				   .link (0)
				   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				   .work (*node0->work_generate_blocking (change1->hash ()))
				   .build ();

	// send Gxrb_ratio from genesis to key1 and genesis rep back to genesis account
	auto send2 = builder
				 .make_block ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (change2->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key1.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*node0->work_generate_blocking (change2->hash ()))
				 .build ();

	// receive send2 and rep of key1 to be itself
	auto receive2 = builder
					.make_block ()
					.account (key1.pub)
					.previous (0)
					.representative (key1.pub)
					.balance (nano::Gxrb_ratio)
					.link (send2->hash ())
					.sign (key1.prv, key1.pub)
					.work (*node0->work_generate_blocking (key1.pub))
					.build ();

	// send Gxrb_ratio raw, all available balance, from key1 to key2
	auto send3 = builder
				 .make_block ()
				 .account (key1.pub)
				 .previous (receive2->hash ())
				 .representative (key1.pub)
				 .balance (0)
				 .link (key2.pub)
				 .sign (key1.prv, key1.pub)
				 .work (*node0->work_generate_blocking (receive2->hash ()))
				 .build ();

	// receive send3 on key2, set rep of key2 to be itself
	auto receive3 = builder
					.make_block ()
					.account (key2.pub)
					.previous (0)
					.representative (key2.pub)
					.balance (nano::Gxrb_ratio)
					.link (send3->hash ())
					.sign (key2.prv, key2.pub)
					.work (*node0->work_generate_blocking (key2.pub))
					.build ();

	std::vector<std::shared_ptr<nano::block>> blocks = { send1, receive1, change1, change2, send2, receive2, send3, receive3 };
	ASSERT_TRUE (nano::test::process (*node0, blocks));
	nano::test::confirm (node0->ledger, blocks);

	config.peering_port = system.get_available_port ();
	auto node1 = system.make_disconnected_node (config, node_flags);

	// Processing chain to prune for node1
	node1->process_active (send1);
	node1->process_active (receive1);
	node1->process_active (change1);
	node1->process_active (change2);
	ASSERT_TIMELY (5s, nano::test::exists (*node1, { send1, receive1, change1, change2 }));

	// Confirm last block to prune previous
	nano::test::confirm (node1->ledger, { send1, receive1, change1, change2 });
	ASSERT_EQ (5, node1->ledger.block_count ());
	ASSERT_EQ (5, node1->ledger.cemented_count ());

	// Pruning action
	node1->ledger_pruning (2, false);
	ASSERT_EQ (9, node0->ledger.block_count ());
	ASSERT_EQ (0, node0->ledger.pruned_count ());
	ASSERT_EQ (5, node1->ledger.block_count ());
	ASSERT_EQ (3, node1->ledger.pruned_count ());

	// Start lazy bootstrap with last block in chain known
	nano::test::establish_tcp (system, *node1, node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap_lazy (receive3->hash (), true);

	// Check processed blocks
	ASSERT_TIMELY_EQ (5s, node1->ledger.block_count (), 9);
	ASSERT_TIMELY (5s, node1->balance (key2.pub) != 0);
	ASSERT_TIMELY (5s, !node1->bootstrap_initiator.in_progress ());
}

TEST (bootstrap_processor, lazy_max_pull_count)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	auto node0 (system.add_node (config, node_flags));
	nano::keypair key1;
	nano::keypair key2;
	// Generating test chain

	nano::state_block_builder builder;

	auto send1 = builder
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key1.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*node0->work_generate_blocking (nano::dev::genesis->hash ()))
				 .build ();
	auto receive1 = builder
					.make_block ()
					.account (key1.pub)
					.previous (0)
					.representative (key1.pub)
					.balance (nano::Gxrb_ratio)
					.link (send1->hash ())
					.sign (key1.prv, key1.pub)
					.work (*node0->work_generate_blocking (key1.pub))
					.build ();
	auto send2 = builder
				 .make_block ()
				 .account (key1.pub)
				 .previous (receive1->hash ())
				 .representative (key1.pub)
				 .balance (0)
				 .link (key2.pub)
				 .sign (key1.prv, key1.pub)
				 .work (*node0->work_generate_blocking (receive1->hash ()))
				 .build ();
	auto receive2 = builder
					.make_block ()
					.account (key2.pub)
					.previous (0)
					.representative (key2.pub)
					.balance (nano::Gxrb_ratio)
					.link (send2->hash ())
					.sign (key2.prv, key2.pub)
					.work (*node0->work_generate_blocking (key2.pub))
					.build ();
	auto change1 = builder
				   .make_block ()
				   .account (key2.pub)
				   .previous (receive2->hash ())
				   .representative (key1.pub)
				   .balance (nano::Gxrb_ratio)
				   .link (0)
				   .sign (key2.prv, key2.pub)
				   .work (*node0->work_generate_blocking (receive2->hash ()))
				   .build ();
	auto change2 = builder
				   .make_block ()
				   .account (key2.pub)
				   .previous (change1->hash ())
				   .representative (nano::dev::genesis_key.pub)
				   .balance (nano::Gxrb_ratio)
				   .link (0)
				   .sign (key2.prv, key2.pub)
				   .work (*node0->work_generate_blocking (change1->hash ()))
				   .build ();
	auto change3 = builder
				   .make_block ()
				   .account (key2.pub)
				   .previous (change2->hash ())
				   .representative (key2.pub)
				   .balance (nano::Gxrb_ratio)
				   .link (0)
				   .sign (key2.prv, key2.pub)
				   .work (*node0->work_generate_blocking (change2->hash ()))
				   .build ();
	// Processing test chain
	node0->block_processor.add (send1);
	node0->block_processor.add (receive1);
	node0->block_processor.add (send2);
	node0->block_processor.add (receive2);
	node0->block_processor.add (change1);
	node0->block_processor.add (change2);
	node0->block_processor.add (change3);
	ASSERT_TIMELY (5s, nano::test::exists (*node0, { send1, receive1, send2, receive2, change1, change2, change3 }));

	// Start lazy bootstrap with last block in chain known
	auto node1 = system.make_disconnected_node ();
	nano::test::establish_tcp (system, *node1, node0->network->endpoint ());
	node1->bootstrap_initiator.bootstrap_lazy (change3->hash ());
	// Check processed blocks
	ASSERT_TIMELY (10s, node1->block (change3->hash ()));
}

TEST (bootstrap_processor, lazy_unclear_state_link)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_disable_legacy_bootstrap (true);
	node_flags.set_disable_ascending_bootstrap (true);
	node_flags.set_disable_ongoing_bootstrap (true);
	auto node1 = system.add_node (config, node_flags);
	nano::keypair key;

	nano::block_builder builder;

	auto send1 = builder
				 .state ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (nano::dev::genesis->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send1));
	auto send2 = builder
				 .state ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (send1->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - 2 * nano::Gxrb_ratio)
				 .link (key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (send1->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send2));
	auto open = builder
				.open ()
				.source (send1->hash ())
				.representative (key.pub)
				.account (key.pub)
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (open));
	auto receive = builder
				   .state ()
				   .account (key.pub)
				   .previous (open->hash ())
				   .representative (key.pub)
				   .balance (2 * nano::Gxrb_ratio)
				   .link (send2->hash ())
				   .sign (key.prv, key.pub)
				   .work (*system.work.generate (open->hash ()))
				   .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (receive));

	ASSERT_TIMELY (5s, nano::test::exists (*node1, { send1, send2, open, receive }));

	// Start lazy bootstrap with last block in chain known
	auto node2 = system.make_disconnected_node (std::nullopt, node_flags);
	nano::test::establish_tcp (system, *node2, node1->network->endpoint ());
	node2->bootstrap_initiator.bootstrap_lazy (receive->hash ());
	ASSERT_TIMELY (5s, nano::test::exists (*node2, { send1, send2, open, receive }));
	ASSERT_EQ (0, node2->stats->count (nano::stat::type::bootstrap, nano::stat::detail::bulk_pull_failed_account, nano::stat::dir::in));
	node2->stop ();
}

TEST (bootstrap_processor, lazy_unclear_state_link_not_existing)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_disable_legacy_bootstrap (true);
	node_flags.set_disable_ascending_bootstrap (true);
	node_flags.set_disable_ongoing_bootstrap (true);
	auto node1 = system.add_node (config, node_flags);
	nano::keypair key, key2;
	// Generating test chain

	nano::block_builder builder;

	auto send1 = builder
				 .state ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (nano::dev::genesis->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send1));
	auto open = builder
				.open ()
				.source (send1->hash ())
				.representative (key.pub)
				.account (key.pub)
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (open));
	auto send2 = builder
				 .state ()
				 .account (key.pub)
				 .previous (open->hash ())
				 .representative (key.pub)
				 .balance (0)
				 .link (key2.pub)
				 .sign (key.prv, key.pub)
				 .work (*system.work.generate (open->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send2));

	// Start lazy bootstrap with last block in chain known
	auto node2 = system.make_disconnected_node (std::nullopt, node_flags);
	nano::test::establish_tcp (system, *node2, node1->network->endpoint ());
	node2->bootstrap_initiator.bootstrap_lazy (send2->hash ());
	// Check processed blocks
	ASSERT_TIMELY (15s, !node2->bootstrap_initiator.in_progress ());
	ASSERT_TIMELY (15s, nano::test::block_or_pruned_all_exists (*node2, { send1, open, send2 }));
	ASSERT_EQ (1, node2->stats->count (nano::stat::type::bootstrap, nano::stat::detail::bulk_pull_failed_account, nano::stat::dir::in));
	node2->stop ();
}

TEST (bootstrap_processor, lazy_destinations)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_disable_legacy_bootstrap (true);
	node_flags.set_disable_ascending_bootstrap (true);
	node_flags.set_disable_ongoing_bootstrap (true);
	auto node1 = system.add_node (config, node_flags);
	nano::keypair key1, key2;

	nano::block_builder builder;

	// send Gxrb_ratio raw from genesis to key1
	auto send1 = builder
				 .state ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key1.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (nano::dev::genesis->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send1));

	// send Gxrb_ratio raw from genesis to key2
	auto send2 = builder
				 .state ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (send1->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - 2 * nano::Gxrb_ratio)
				 .link (key2.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (send1->hash ()))
				 .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (send2));

	// receive send1 on key1
	auto open = builder
				.open ()
				.source (send1->hash ())
				.representative (key1.pub)
				.account (key1.pub)
				.sign (key1.prv, key1.pub)
				.work (*system.work.generate (key1.pub))
				.build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (open));

	// receive send2 on key2
	auto state_open = builder
					  .state ()
					  .account (key2.pub)
					  .previous (0)
					  .representative (key2.pub)
					  .balance (nano::Gxrb_ratio)
					  .link (send2->hash ())
					  .sign (key2.prv, key2.pub)
					  .work (*system.work.generate (key2.pub))
					  .build ();
	ASSERT_EQ (nano::block_status::progress, node1->process (state_open));

	// Start lazy bootstrap with last block in sender chain
	auto node2 = system.make_disconnected_node (std::nullopt, node_flags);
	nano::test::establish_tcp (system, *node2, node1->network->endpoint ());
	node2->bootstrap_initiator.bootstrap_lazy (send2->hash ());

	// Check processed blocks
	ASSERT_TIMELY (5s, !node2->bootstrap_initiator.in_progress ());
	ASSERT_TIMELY (5s, node2->block_or_pruned_exists (send1->hash ()));
	ASSERT_TIMELY (5s, node2->block_or_pruned_exists (send2->hash ()));
	ASSERT_FALSE (node2->block_or_pruned_exists (open->hash ()));
	ASSERT_FALSE (node2->block_or_pruned_exists (state_open->hash ()));
	node2->stop ();
}

TEST (bootstrap_processor, wallet_lazy_pending)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_disable_legacy_bootstrap (true);
	node_flags.set_disable_ascending_bootstrap (true);
	node_flags.set_disable_ongoing_bootstrap (true);
	auto node0 = system.add_node (config, node_flags);
	nano::keypair key1;
	nano::keypair key2;
	// Generating test chain

	nano::state_block_builder builder;

	auto send1 = builder
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key1.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*node0->work_generate_blocking (nano::dev::genesis->hash ()))
				 .build ();
	auto receive1 = builder
					.make_block ()
					.account (key1.pub)
					.previous (0)
					.representative (key1.pub)
					.balance (nano::Gxrb_ratio)
					.link (send1->hash ())
					.sign (key1.prv, key1.pub)
					.work (*node0->work_generate_blocking (key1.pub))
					.build ();
	auto send2 = builder
				 .make_block ()
				 .account (key1.pub)
				 .previous (receive1->hash ())
				 .representative (key1.pub)
				 .balance (0)
				 .link (key2.pub)
				 .sign (key1.prv, key1.pub)
				 .work (*node0->work_generate_blocking (receive1->hash ()))
				 .build ();

	// Processing test chain
	node0->block_processor.add (send1);
	node0->block_processor.add (receive1);
	node0->block_processor.add (send2);
	nano::test::exists (*node0, { send1, receive1, send2 });

	// Start wallet lazy bootstrap
	auto node1 = system.add_node ();
	nano::test::establish_tcp (system, *node1, node0->network->endpoint ());
	auto wallet_id{ nano::random_wallet_id () };
	node1->wallets.create (wallet_id);
	nano::account account;
	ASSERT_EQ (nano::wallets_error::none, node1->wallets.insert_adhoc (wallet_id, key2.prv, true, account));
	node1->bootstrap_wallet ();
	// Check processed blocks
	ASSERT_TIMELY (10s, node1->block_or_pruned_exists (send2->hash ()));
}

TEST (bulk, genesis)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_disable_lazy_bootstrap (true);
	auto node1 = system.add_node (config, node_flags);
	auto wallet_id = node1->wallets.first_wallet_id ();
	(void)node1->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);

	auto node2 = system.make_disconnected_node ();
	nano::block_hash latest1 (node1->latest (nano::dev::genesis_key.pub));
	nano::block_hash latest2 (node2->latest (nano::dev::genesis_key.pub));
	ASSERT_EQ (latest1, latest2);
	nano::keypair key2;
	auto send (node1->wallets.send_action (wallet_id, nano::dev::genesis_key.pub, key2.pub, 100));
	ASSERT_NE (nullptr, send);
	nano::block_hash latest3 (node1->latest (nano::dev::genesis_key.pub));
	ASSERT_NE (latest1, latest3);

	node2->connect (node1->network->endpoint ());
	node2->bootstrap_initiator.bootstrap (node1->network->endpoint ());
	ASSERT_TIMELY_EQ (10s, node2->latest (nano::dev::genesis_key.pub), node1->latest (nano::dev::genesis_key.pub));
	ASSERT_EQ (node2->latest (nano::dev::genesis_key.pub), node1->latest (nano::dev::genesis_key.pub));
	node2->stop ();
}

TEST (bulk, offline_send)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_disable_lazy_bootstrap (true);

	auto node1 = system.add_node (config, node_flags);
	auto wallet_id = node1->wallets.first_wallet_id ();
	(void)node1->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	const auto amount = node1->config->receive_minimum.number ();
	auto node2 = system.make_disconnected_node ();
	nano::keypair key2;
	auto wallet_id2{ nano::random_wallet_id () };
	node2->wallets.create (wallet_id2);
	nano::account account;
	ASSERT_EQ (nano::wallets_error::none, node2->wallets.insert_adhoc (wallet_id2, key2.prv, true, account));

	// send amount from genesis to key2, it will be autoreceived
	auto wallet_id1 = node1->wallets.first_wallet_id ();
	auto send1 (node1->wallets.send_action (wallet_id1, nano::dev::genesis_key.pub, key2.pub, node1->config->receive_minimum.number ()));
	ASSERT_NE (nullptr, send1);

	// Wait to finish election background tasks
	ASSERT_TIMELY (5s, node1->active.empty ());
	ASSERT_TIMELY (5s, node1->block_confirmed (send1->hash ()));
	ASSERT_EQ (std::numeric_limits<nano::uint128_t>::max () - amount, node1->balance (nano::dev::genesis_key.pub));

	// Initiate bootstrap
	node2->connect (node1->network->endpoint ());
	node2->bootstrap_initiator.bootstrap (node1->network->endpoint ());

	// Nodes should find each other after bootstrap initiation
	ASSERT_TIMELY (5s, !node1->network->empty ());
	ASSERT_TIMELY (5s, !node2->network->empty ());

	// Send block arrival via bootstrap
	ASSERT_TIMELY_EQ (5s, node2->balance (nano::dev::genesis_key.pub), std::numeric_limits<nano::uint128_t>::max () - amount);
	// Receiving send block
	ASSERT_TIMELY_EQ (5s, node2->balance (key2.pub), amount);
	node2->stop ();
}

TEST (bulk, genesis_pruning)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	config.enable_voting = false; // Remove after allowing pruned voting
	nano::node_flags node_flags;
	node_flags.set_disable_bootstrap_bulk_push_client (true);
	node_flags.set_disable_lazy_bootstrap (true);
	node_flags.set_disable_ongoing_bootstrap (true);
	node_flags.set_disable_ascending_bootstrap (true);
	node_flags.set_enable_pruning (true);

	auto node1 = system.add_node (config, node_flags);
	auto blocks = nano::test::setup_chain (system, *node1, 3);
	auto send1 = blocks[0];
	auto send2 = blocks[1];
	auto send3 = blocks[2];

	ASSERT_EQ (4, node1->ledger.block_count ());

	node1->ledger_pruning (2, false);
	ASSERT_EQ (2, node1->ledger.pruned_count ());
	ASSERT_EQ (4, node1->ledger.block_count ());
	ASSERT_TRUE (node1->ledger.store.pruned ().exists (*node1->ledger.store.tx_begin_read (), send1->hash ()));
	ASSERT_FALSE (nano::test::exists (*node1, { send1 }));
	ASSERT_TRUE (node1->ledger.store.pruned ().exists (*node1->ledger.store.tx_begin_read (), send2->hash ()));
	ASSERT_FALSE (nano::test::exists (*node1, { send2 }));
	ASSERT_TRUE (nano::test::exists (*node1, { send3 }));

	// Bootstrap with missing blocks for node2
	node_flags.set_enable_pruning (false);
	auto node2 = system.make_disconnected_node (std::nullopt, node_flags);
	node2->connect (node1->network->endpoint ());
	node2->bootstrap_initiator.bootstrap (node1->network->endpoint ());
	node2->network->merge_peer (node1->network->endpoint ());
	ASSERT_TIMELY (5s, node2->stats->count (nano::stat::type::bootstrap, nano::stat::detail::initiate, nano::stat::dir::out) >= 1);
	ASSERT_TIMELY (5s, !node2->bootstrap_initiator.in_progress ());

	// node2 still missing blocks
	ASSERT_EQ (1, node2->ledger.block_count ());
	{
		auto transaction (node2->store.tx_begin_write ());
		node2->unchecked.clear ();
	}

	// Insert pruned blocks
	node2->process_active (send1);
	node2->process_active (send2);
	ASSERT_TIMELY_EQ (5s, 3, node2->ledger.block_count ());

	// New bootstrap to sync up everything
	ASSERT_TIMELY_EQ (5s, node2->bootstrap_initiator.connections->get_connections_count (), 0);
	node2->connect (node1->network->endpoint ());
	node2->bootstrap_initiator.bootstrap (node1->network->endpoint ());
	ASSERT_TIMELY_EQ (5s, node2->latest (nano::dev::genesis_key.pub), node1->latest (nano::dev::genesis_key.pub));
	node2->stop ();
}
