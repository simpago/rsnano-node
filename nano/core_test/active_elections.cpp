#include <nano/lib/blocks.hpp>
#include <nano/lib/jsonconfig.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/election.hpp>
#include <nano/node/scheduler/component.hpp>
#include <nano/node/scheduler/manual.hpp>
#include <nano/node/scheduler/priority.hpp>
#include <nano/secure/common.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/test_common/chains.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <thread>

using namespace std::chrono_literals;

namespace nano
{
TEST (active_elections, vote_replays)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.enable_voting = false;
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto & node = *system.add_node (node_config);
	nano::keypair key;
	nano::state_block_builder builder;

	// send Gxrb_ratio raw from genesis to key
	auto send1 = builder.make_block ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (nano::dev::genesis->hash ()))
				 .build ();
	ASSERT_NE (nullptr, send1);

	// create open block for key receing Gxrb_ratio raw
	auto open1 = builder.make_block ()
				 .account (key.pub)
				 .previous (0)
				 .representative (key.pub)
				 .balance (nano::Gxrb_ratio)
				 .link (send1->hash ())
				 .sign (key.prv, key.pub)
				 .work (*system.work.generate (key.pub))
				 .build ();
	ASSERT_NE (nullptr, open1);

	// wait for elections objects to appear in the AEC
	node.process_active (send1);
	node.process_active (open1);
	ASSERT_TRUE (nano::test::start_elections (system, node, { send1, open1 }));
	ASSERT_EQ (2, node.active.size ());

	// First vote is not a replay and confirms the election, second vote should be a replay since the election has confirmed but not yet removed
	auto vote_send1 = nano::test::make_final_vote (nano::dev::genesis_key, { send1 });
	ASSERT_EQ (nano::vote_code::vote, node.vote (*vote_send1, send1->hash ()));
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote_send1, send1->hash ()));

	// Wait until the election is removed, at which point the vote is still a replay since it's been recently confirmed
	ASSERT_TIMELY_EQ (5s, node.active.size (), 1);
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote_send1, send1->hash ()));

	// Open new account
	auto vote_open1 = nano::test::make_final_vote (nano::dev::genesis_key, { open1 });
	ASSERT_EQ (nano::vote_code::vote, node.vote (*vote_open1, open1->hash ()));
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote_open1, open1->hash ()));
	ASSERT_TIMELY (5s, node.active.empty ());
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote_open1, open1->hash ()));
	ASSERT_EQ (nano::Gxrb_ratio, node.ledger.weight (key.pub));

	// send 1 raw to key to key
	auto send2 = builder.make_block ()
				 .account (key.pub)
				 .previous (open1->hash ())
				 .representative (key.pub)
				 .balance (nano::Gxrb_ratio - 1)
				 .link (key.pub)
				 .sign (key.prv, key.pub)
				 .work (*system.work.generate (open1->hash ()))
				 .build ();
	ASSERT_NE (nullptr, send2);
	node.process_active (send2);
	ASSERT_TRUE (nano::test::start_elections (system, node, { send2 }));
	ASSERT_EQ (1, node.active.size ());

	// vote2_send2 is a non final vote with little weight, vote1_send2 is the vote that confirms the election
	auto vote1_send2 = nano::test::make_final_vote (nano::dev::genesis_key, { send2 });
	auto vote2_send2 = nano::test::make_vote (key, { send2 }, 0, 0);
	ASSERT_EQ (nano::vote_code::vote, node.vote (*vote2_send2, send2->hash ())); // this vote cannot confirm the election
	ASSERT_EQ (1, node.active.size ());
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote2_send2, send2->hash ())); // this vote cannot confirm the election
	ASSERT_EQ (1, node.active.size ());
	ASSERT_EQ (nano::vote_code::vote, node.vote (*vote1_send2, send2->hash ())); // this vote confirms the election

	// this should still return replay, either because the election is still in the AEC or because it is recently confirmed
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote1_send2, send2->hash ()));
	ASSERT_TIMELY (5s, node.active.empty ());
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote1_send2, send2->hash ()));
	ASSERT_EQ (nano::vote_code::replay, node.vote (*vote2_send2, send2->hash ()));

	// Removing blocks as recently confirmed makes every vote indeterminate
	node.active.clear_recently_confirmed ();
	ASSERT_EQ (nano::vote_code::indeterminate, node.vote (*vote_send1, send1->hash ()));
	ASSERT_EQ (nano::vote_code::indeterminate, node.vote (*vote_open1, open1->hash ()));
	ASSERT_EQ (nano::vote_code::indeterminate, node.vote (*vote1_send2, send2->hash ()));
	ASSERT_EQ (nano::vote_code::indeterminate, node.vote (*vote2_send2, send2->hash ()));
}
}

// Tests that blocks are correctly cleared from the duplicate filter for unconfirmed elections
TEST (active_elections, dropped_cleanup)
{
	nano::test::system system;
	nano::node_flags flags;
	flags.set_disable_request_loop (true);
	auto & node (*system.add_node (flags));
	auto chain = nano::test::setup_chain (system, node, 1, nano::dev::genesis_key, false);
	auto hash = chain[0]->hash ();

	// Add to network filter to ensure proper cleanup after the election is dropped
	std::vector<uint8_t> block_bytes;
	{
		nano::vectorstream stream (block_bytes);
		chain[0]->serialize (stream);
	}
	ASSERT_FALSE (node.network->tcp_channels->publish_filter->apply (block_bytes.data (), block_bytes.size ()));
	ASSERT_TRUE (node.network->tcp_channels->publish_filter->apply (block_bytes.data (), block_bytes.size ()));

	auto election = nano::test::start_election (system, node, hash);
	ASSERT_NE (nullptr, election);

	// Not yet removed
	ASSERT_TRUE (node.network->tcp_channels->publish_filter->apply (block_bytes.data (), block_bytes.size ()));
	ASSERT_TRUE (node.election_active (hash));

	// Now simulate dropping the election
	ASSERT_FALSE (node.active.confirmed (*election));
	node.active.erase (*chain[0]);

	// The filter must have been cleared
	ASSERT_FALSE (node.network->tcp_channels->publish_filter->apply (block_bytes.data (), block_bytes.size ()));

	// An election was recently dropped
	ASSERT_EQ (1, node.stats->count (nano::stat::type::active_elections_dropped, nano::stat::detail::manual));

	// Block cleared from active
	ASSERT_FALSE (node.election_active (hash));

	// Repeat test for a confirmed election
	ASSERT_TRUE (node.network->tcp_channels->publish_filter->apply (block_bytes.data (), block_bytes.size ()));

	election = nano::test::start_election (system, node, hash);
	ASSERT_NE (nullptr, election);
	node.active.force_confirm (*election);
	ASSERT_TIMELY (5s, node.active.confirmed (*election));
	node.active.erase (*chain[0]);

	// The filter should not have been cleared
	ASSERT_TRUE (node.network->tcp_channels->publish_filter->apply (block_bytes.data (), block_bytes.size ()));

	// Not dropped
	ASSERT_EQ (1, node.stats->count (nano::stat::type::active_elections_dropped, nano::stat::detail::manual));

	// Block cleared from active
	ASSERT_FALSE (node.election_active (hash));
}

TEST (active_elections, fork_filter_cleanup)
{
	nano::test::system system{};

	nano::node_config node_config = system.default_config ();
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;

	auto & node1 = *system.add_node (node_config);
	nano::keypair key{};
	nano::state_block_builder builder{};
	auto const latest_hash = nano::dev::genesis->hash ();

	auto send1 = builder.make_block ()
				 .previous (latest_hash)
				 .account (nano::dev::genesis_key.pub)
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (latest_hash))
				 .build ();

	std::vector<uint8_t> send_block_bytes{};
	{
		nano::vectorstream stream{ send_block_bytes };
		send1->serialize (stream);
	}

	// Generate 10 forks to prevent new block insertion to election
	for (auto i = 0; i < 10; ++i)
	{
		auto fork = builder.make_block ()
					.previous (latest_hash)
					.account (nano::dev::genesis_key.pub)
					.representative (nano::dev::genesis_key.pub)
					.balance (nano::dev::constants.genesis_amount - 1 - i)
					.link (key.pub)
					.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					.work (*system.work.generate (latest_hash))
					.build ();

		node1.process_active (fork);
		ASSERT_TIMELY (5s, node1.active.election (fork->qualified_root ()) != nullptr);
	}

	// All forks were merged into the same election
	std::shared_ptr<nano::election> election{};
	ASSERT_TIMELY (5s, (election = node1.active.election (send1->qualified_root ())) != nullptr);
	ASSERT_TIMELY_EQ (5s, election->blocks ().size (), 10);
	ASSERT_EQ (1, node1.active.size ());

	// Instantiate a new node
	node_config.peering_port = system.get_available_port ();
	auto & node2 = *system.add_node (node_config);

	// Process the first initial block on node2
	node2.process_active (send1);
	ASSERT_TIMELY (5s, node2.active.election (send1->qualified_root ()) != nullptr);

	// TODO: questions: why doesn't node2 pick up "fork" from node1? because it connected to node1 after node1
	//                  already process_active()d the fork? shouldn't it broadcast it anyway, even later?
	//
	//                  how about node1 picking up "send1" from node2? we know it does because we assert at
	//                  the end that it is within node1's AEC, but why node1.block_count doesn't increase?
	//
	ASSERT_TIMELY_EQ (5s, node2.ledger.block_count (), 2);
	ASSERT_TIMELY_EQ (5s, node1.ledger.block_count (), 2);

	// Block is erased from the duplicate filter
	ASSERT_TIMELY (5s, node1.network->tcp_channels->publish_filter->apply (send_block_bytes.data (), send_block_bytes.size ()));
}

/*
 * What this test is doing:
 * Create 20 representatives with minimum principal weight each
 * Create a send block on the genesis account (the last send block)
 * Create 20 forks of the last send block using genesis as representative (no votes produced)
 * Check that only 10 blocks remain in the election (due to max 10 forks per election object limit)
 * Create 20 more forks of the last send block using the new reps as representatives and produce votes for them
 *     (9 votes from this batch should survive and replace existing blocks in the election, why not 10?)
 * Then send winning block and it should replace one of the existing blocks
 */
// Disabled by Gustav. It is flaky.
TEST (active_elections, DISABLED_fork_replacement_tally)
{
	// TODO reimplement in Rust
}

namespace nano
{
// Blocks that won an election must always be seen as confirming or cemented
TEST (active_elections, confirmation_consistency)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto & node = *system.add_node (node_config);
	auto wallet_id = node.wallets.first_wallet_id ();
	(void)node.wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	for (unsigned i = 0; i < 10; ++i)
	{
		auto block (node.wallets.send_action (wallet_id, nano::dev::genesis_key.pub, nano::public_key (), node.config->receive_minimum.number ()));
		ASSERT_NE (nullptr, block);
		ASSERT_TIMELY (5s, node.block_confirmed (block->hash ()));
		ASSERT_NO_ERROR (system.poll_until_true (1s, [&node, &block, i] {
			EXPECT_EQ (i + 1, node.active.recently_confirmed_size ());
			EXPECT_EQ (block->qualified_root (), node.active.lastest_recently_confirmed_root ());
			return i + 1 == node.active.recently_cemented_size (); // done after a callback
		}));
	}
}
}

TEST (active_elections, confirm_new)
{
	nano::test::system system (1);
	auto & node1 = *system.nodes[0];
	auto send = nano::send_block_builder ()
				.previous (nano::dev::genesis->hash ())
				.destination (nano::public_key ())
				.balance (nano::dev::constants.genesis_amount - 100)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();
	node1.process_active (send);
	ASSERT_TIMELY_EQ (5s, 1, node1.active.size ());
	auto & node2 = *system.add_node ();
	// Add key to node2
	auto wallet_id = node2.wallets.first_wallet_id ();
	(void)node2.wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	// Let node2 know about the block
	auto send_copy = nano::send_block_builder ().make_block ().from (*send).build ();
	ASSERT_TIMELY (5s, node2.block (send_copy->hash ()));
	// Wait confirmation
	ASSERT_TIMELY (5s, node1.ledger.cemented_count () == 2);
	ASSERT_TIMELY (5s, node2.ledger.cemented_count () == 2);
}

// Ensures votes are tallied on election::publish even if no vote is inserted through inactive_votes_cache
TEST (active_elections, conflicting_block_vote_existing_election)
{
	nano::test::system system;
	nano::node_flags node_flags;
	node_flags.set_disable_request_loop (true);
	auto & node = *system.add_node (node_flags);
	nano::keypair key;
	nano::state_block_builder builder;
	auto send = builder.make_block ()
				.account (nano::dev::genesis_key.pub)
				.previous (nano::dev::genesis->hash ())
				.representative (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - 100)
				.link (key.pub)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();
	auto fork = builder.make_block ()
				.account (nano::dev::genesis_key.pub)
				.previous (nano::dev::genesis->hash ())
				.representative (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - 200)
				.link (key.pub)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();
	auto vote_fork = nano::test::make_final_vote (nano::dev::genesis_key, { fork });

	ASSERT_EQ (nano::block_status::progress, node.process_local (send).value ());
	ASSERT_TIMELY_EQ (5s, 1, node.active.size ());

	// Vote for conflicting block, but the block does not yet exist in the ledger
	node.vote (*vote_fork);

	// Block now gets processed
	ASSERT_EQ (nano::block_status::fork, node.process_local (fork).value ());

	// Election must be confirmed
	auto election (node.active.election (fork->qualified_root ()));
	ASSERT_NE (nullptr, election);
	ASSERT_TIMELY (3s, node.active.confirmed (*election));
}

// This tests the node's internal block activation logic
TEST (active_elections, activate_account_chain)
{
	nano::test::system system;
	nano::node_flags flags;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto & node = *system.add_node (config, flags);

	nano::keypair key;
	nano::state_block_builder builder;
	auto send = builder.make_block ()
				.account (nano::dev::genesis_key.pub)
				.previous (nano::dev::genesis->hash ())
				.representative (nano::dev::genesis_key.pub)
				.link (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - 1)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();
	auto send2 = builder.make_block ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (send->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .link (key.pub)
				 .balance (nano::dev::constants.genesis_amount - 2)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (send->hash ()))
				 .build ();
	auto send3 = builder.make_block ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (send2->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .link (key.pub)
				 .balance (nano::dev::constants.genesis_amount - 3)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (send2->hash ()))
				 .build ();
	auto open = builder.make_block ()
				.account (key.pub)
				.previous (0)
				.representative (key.pub)
				.link (send2->hash ())
				.balance (1)
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();
	auto receive = builder.make_block ()
				   .account (key.pub)
				   .previous (open->hash ())
				   .representative (key.pub)
				   .link (send3->hash ())
				   .balance (2)
				   .sign (key.prv, key.pub)
				   .work (*system.work.generate (open->hash ()))
				   .build ();
	ASSERT_EQ (nano::block_status::progress, node.process (send));
	ASSERT_EQ (nano::block_status::progress, node.process (send2));
	ASSERT_EQ (nano::block_status::progress, node.process (send3));
	ASSERT_EQ (nano::block_status::progress, node.process (open));
	ASSERT_EQ (nano::block_status::progress, node.process (receive));

	auto election1 = nano::test::start_election (system, node, send->hash ());
	ASSERT_EQ (1, node.active.size ());
	ASSERT_EQ (1, election1->blocks ().count (send->hash ()));
	node.active.force_confirm (*election1); // Force confirm to trigger successor activation
	ASSERT_TIMELY (3s, node.block_confirmed (send->hash ()));
	// On cementing, the next election is started
	ASSERT_TIMELY (3s, node.active.active (send2->qualified_root ()));
	auto election3 = node.active.election (send2->qualified_root ());
	ASSERT_NE (nullptr, election3);
	ASSERT_EQ (1, election3->blocks ().count (send2->hash ()));
	node.active.force_confirm (*election3); // Force confirm to trigger successor and destination activation
	ASSERT_TIMELY (3s, node.block_confirmed (send2->hash ()));
	// On cementing, the next election is started
	ASSERT_TIMELY (3s, node.active.active (open->qualified_root ())); // Destination account activated
	ASSERT_TIMELY (3s, node.active.active (send3->qualified_root ())); // Block successor activated
	auto election4 = node.active.election (send3->qualified_root ());
	ASSERT_NE (nullptr, election4);
	ASSERT_EQ (1, election4->blocks ().count (send3->hash ()));
	auto election5 = node.active.election (open->qualified_root ());
	ASSERT_NE (nullptr, election5);
	ASSERT_EQ (1, election5->blocks ().count (open->hash ()));
	node.active.force_confirm (*election5);
	ASSERT_TIMELY (3s, node.block_confirmed (open->hash ()));
	// Until send3 is also confirmed, the receive block should not activate
	std::this_thread::sleep_for (200ms);
	ASSERT_FALSE (node.active.active (receive->qualified_root ()));
	node.active.force_confirm (*election4);
	ASSERT_TIMELY (3s, node.block_confirmed (send3->hash ()));
	ASSERT_TIMELY (3s, node.active.active (receive->qualified_root ())); // Destination account activated
}

TEST (active_elections, activate_inactive)
{
	nano::test::system system;
	nano::node_flags flags;
	nano::node_config config = system.default_config ();
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto & node = *system.add_node (config, flags);

	nano::keypair key;
	nano::state_block_builder builder;
	auto send = builder.make_block ()
				.account (nano::dev::genesis_key.pub)
				.previous (nano::dev::genesis->hash ())
				.representative (nano::dev::genesis_key.pub)
				.link (key.pub)
				.balance (nano::dev::constants.genesis_amount - 1)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();
	auto send2 = builder.make_block ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (send->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .link (nano::keypair ().pub)
				 .balance (nano::dev::constants.genesis_amount - 2)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (send->hash ()))
				 .build ();
	auto open = builder.make_block ()
				.account (key.pub)
				.previous (0)
				.representative (key.pub)
				.link (send->hash ())
				.balance (1)
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();

	ASSERT_EQ (nano::block_status::progress, node.process (send));
	ASSERT_EQ (nano::block_status::progress, node.process (send2));
	ASSERT_EQ (nano::block_status::progress, node.process (open));

	auto election = nano::test::start_election (system, node, send2->hash ());
	ASSERT_NE (nullptr, election);
	node.active.force_confirm (*election);

	ASSERT_TIMELY (5s, !node.confirming_set.exists (send2->hash ()));
	ASSERT_TIMELY (5s, node.block_confirmed (send2->hash ()));
	ASSERT_TIMELY (5s, node.block_confirmed (send->hash ()));

	// wait so that blocks observer can increase the stats
	std::this_thread::sleep_for (1000ms);

	ASSERT_TIMELY_EQ (5s, 1, node.stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::inactive_conf_height, nano::stat::dir::out));
	ASSERT_TIMELY_EQ (5s, 1, node.stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::active_quorum, nano::stat::dir::out));
	ASSERT_ALWAYS_EQ (50ms, 0, node.stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::active_conf_height, nano::stat::dir::out));

	// The first block was not active so no activation takes place
	ASSERT_FALSE (node.active.active (open->qualified_root ()) || node.block_confirmed_or_being_confirmed (open->hash ()));
}

TEST (active_elections, list_active)
{
	nano::test::system system (1);
	auto & node = *system.nodes[0];

	nano::keypair key;
	nano::state_block_builder builder;
	auto send = builder.make_block ()
				.account (nano::dev::genesis_key.pub)
				.previous (nano::dev::genesis->hash ())
				.representative (nano::dev::genesis_key.pub)
				.link (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - 1)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();

	ASSERT_EQ (nano::block_status::progress, node.process (send));

	auto send2 = builder.make_block ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (send->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .link (key.pub)
				 .balance (nano::dev::constants.genesis_amount - 2)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (send->hash ()))
				 .build ();

	ASSERT_EQ (nano::block_status::progress, node.process (send2));

	auto open = builder.make_block ()
				.account (key.pub)
				.previous (0)
				.representative (key.pub)
				.link (send2->hash ())
				.balance (1)
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();

	ASSERT_EQ (nano::block_status::progress, node.process (open));

	ASSERT_TRUE (nano::test::start_elections (system, node, { send, send2, open }));
	ASSERT_EQ (3, node.active.size ());
	ASSERT_EQ (1, node.active.list_active (1).size ());
	ASSERT_EQ (2, node.active.list_active (2).size ());
	ASSERT_EQ (3, node.active.list_active (3).size ());
	ASSERT_EQ (3, node.active.list_active (4).size ());
	ASSERT_EQ (3, node.active.list_active (99999).size ());
	ASSERT_EQ (3, node.active.list_active ().size ());

	auto active = node.active.list_active ();
}

TEST (active_elections, vacancy)
{
	std::atomic<bool> updated = false;
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.active_elections.size = 1;
	auto & node = *system.add_node (config);
	nano::state_block_builder builder;
	auto send = builder.make_block ()
				.account (nano::dev::genesis_key.pub)
				.previous (nano::dev::genesis->hash ())
				.representative (nano::dev::genesis_key.pub)
				.link (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();
	node.active.set_vacancy_update ([&updated] () { updated = true; });
	ASSERT_EQ (nano::block_status::progress, node.process (send));
	ASSERT_EQ (1, node.active.vacancy (nano::election_behavior::priority));
	ASSERT_EQ (0, node.active.size ());
	auto election1 = nano::test::start_election (system, node, send->hash ());
	ASSERT_TIMELY (1s, updated);
	updated = false;
	ASSERT_EQ (0, node.active.vacancy (nano::election_behavior::priority));
	ASSERT_EQ (1, node.active.size ());
	node.active.force_confirm (*election1);
	ASSERT_TIMELY (1s, updated);
	ASSERT_EQ (1, node.active.vacancy (nano::election_behavior::priority));
	ASSERT_EQ (0, node.active.size ());
}

/*
 * Ensures we limit the number of vote hinted elections in AEC
 */
// disabled because it doesn't run after tokio switch
TEST (DISABLED_active_elections, limit_vote_hinted_elections)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	const int aec_limit = 10;
	config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	config.optimistic_scheduler.enabled = false;
	config.active_elections.size = aec_limit;
	config.active_elections.hinted_limit_percentage = 10; // Should give us a limit of 1 hinted election
	auto & node = *system.add_node (config);

	// Setup representatives
	// Enough weight to trigger election hinting but not enough to confirm block on its own
	const auto amount = ((node.quorum ().trended_weight.number () / 100) * node.config->hinted_scheduler.hinting_threshold_percent) + 1000 * nano::Gxrb_ratio;
	nano::keypair rep1 = nano::test::setup_rep (system, node, amount / 2);
	nano::keypair rep2 = nano::test::setup_rep (system, node, amount / 2);

	auto blocks = nano::test::setup_independent_blocks (system, node, 2);
	auto open0 = blocks[0];
	auto open1 = blocks[1];

	// Even though automatic frontier confirmation is disabled, AEC is doing funny stuff and inserting elections, clear that
	WAIT (1s);
	node.active.clear ();
	ASSERT_TRUE (node.active.empty ());

	// Inactive vote
	auto vote1 = nano::test::make_vote (rep1, { open0, open1 });
	node.vote_processor_queue.vote (vote1, nano::test::fake_channel (node));
	// Ensure new inactive vote cache entries were created
	ASSERT_TIMELY_EQ (5s, node.vote_cache.size (), 2);
	// And no elections are getting started yet
	ASSERT_ALWAYS (1s, node.active.empty ());
	// And nothing got confirmed yet
	ASSERT_FALSE (nano::test::confirmed (node, { open0, open1 }));

	// This vote should trigger election hinting for first receive block
	auto vote2 = nano::test::make_vote (rep2, { open0 });
	node.vote_processor_queue.vote (vote2, nano::test::fake_channel (node));
	// Ensure an election got started for open0 block
	ASSERT_TIMELY_EQ (5s, node.active.size (), 1);
	ASSERT_TIMELY (5s, nano::test::active (node, { open0 }));

	// This vote should trigger election hinting but not become active due to limit of active hinted elections
	auto vote3 = nano::test::make_vote (rep2, { open1 });
	node.vote_processor_queue.vote (vote3, nano::test::fake_channel (node));
	// Ensure no new election are getting started
	ASSERT_NEVER (1s, nano::test::active (node, { open1 }));
	ASSERT_EQ (node.active.size (), 1);

	// This final vote should confirm the first receive block
	auto vote4 = nano::test::make_final_vote (nano::dev::genesis_key, { open0 });
	node.vote_processor_queue.vote (vote4, nano::test::fake_channel (node));
	// Ensure election for open0 block got confirmed
	ASSERT_TIMELY (5s, nano::test::confirmed (node, { open0 }));

	// Now a second block should get vote hinted
	ASSERT_TIMELY (5s, nano::test::active (node, { open1 }));

	std::this_thread::sleep_for (500ms);

	// Ensure there was no overflow of elections
	ASSERT_EQ (0, node.stats->count (nano::stat::type::active_elections_dropped, nano::stat::detail::priority));
}
