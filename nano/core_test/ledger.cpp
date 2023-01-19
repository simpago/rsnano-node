#include <nano/lib/stats.hpp>
#include <nano/lib/threading.hpp>
#include <nano/node/election.hpp>
#include <nano/node/transport/inproc.hpp>
#include <nano/test_common/ledger.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

using namespace std::chrono_literals;

TEST (votes, check_signature)
{
	nano::test::system system;
	nano::node_config node_config (nano::test::get_available_port (), system.logging);
	node_config.online_weight_minimum = std::numeric_limits<nano::uint128_t>::max ();
	auto & node1 = *system.add_node (node_config);
	nano::keypair key1;
	nano::block_builder builder;
	auto send1 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key1.pub)
				 .balance (nano::dev::constants.genesis_amount - 100)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	{
		auto transaction (node1.store.tx_begin_write ());
		ASSERT_EQ (nano::process_result::progress, node1.ledger.process (*transaction, *send1).code);
	}
	node1.scheduler.activate (nano::dev::genesis_key.pub, *node1.store.tx_begin_read ());
	ASSERT_TIMELY (5s, node1.active.election (send1->qualified_root ()));
	auto election1 = node1.active.election (send1->qualified_root ());
	ASSERT_EQ (1, election1->votes ().size ());
	auto vote1 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 1, 0, std::vector<nano::block_hash>{ send1->hash () }));
	vote1->flip_signature_bit_0 ();
	ASSERT_EQ (nano::vote_code::invalid, node1.vote_processor.vote_blocking (vote1, std::make_shared<nano::transport::inproc::channel> (node1, node1)));
	vote1->flip_signature_bit_0 ();
	ASSERT_EQ (nano::vote_code::vote, node1.vote_processor.vote_blocking (vote1, std::make_shared<nano::transport::inproc::channel> (node1, node1)));
	ASSERT_EQ (nano::vote_code::replay, node1.vote_processor.vote_blocking (vote1, std::make_shared<nano::transport::inproc::channel> (node1, node1)));
}

TEST (votes, add_one)
{
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	nano::keypair key1;
	nano::block_builder builder;
	auto send1 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key1.pub)
				 .balance (nano::dev::constants.genesis_amount - 100)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto transaction (node1.store.tx_begin_write ());
	ASSERT_EQ (nano::process_result::progress, node1.ledger.process (*transaction, *send1).code);
	node1.block_confirm (send1);
	ASSERT_TIMELY (5s, node1.active.election (send1->qualified_root ()));
	auto election1 = node1.active.election (send1->qualified_root ());
	ASSERT_EQ (1, election1->votes ().size ());
	auto vote1 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 1, 0, std::vector<nano::block_hash>{ send1->hash () }));
	ASSERT_EQ (nano::vote_code::vote, node1.active.vote (vote1));
	auto vote2 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 2, 0, std::vector<nano::block_hash>{ send1->hash () }));
	ASSERT_EQ (nano::vote_code::vote, node1.active.vote (vote2));
	ASSERT_EQ (2, election1->votes ().size ());
	auto votes1 (election1->votes ());
	auto existing1 (votes1.find (nano::dev::genesis_key.pub));
	ASSERT_NE (votes1.end (), existing1);
	ASSERT_EQ (send1->hash (), existing1->second.hash);
	nano::lock_guard<nano::mutex> guard{ node1.active.mutex };
	auto winner (*election1->tally ().begin ());
	ASSERT_EQ (*send1, *winner.second);
	ASSERT_EQ (nano::dev::constants.genesis_amount - 100, winner.first);
}

namespace nano
{
// Higher timestamps change the vote
TEST (votes, add_existing)
{
	nano::test::system system;
	nano::node_config node_config (nano::test::get_available_port (), system.logging);
	node_config.online_weight_minimum = nano::dev::constants.genesis_amount;
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto & node1 = *system.add_node (node_config);
	nano::keypair key1;
	nano::block_builder builder;
	std::shared_ptr<nano::block> send1 = builder.state ()
										 .account (nano::dev::genesis_key.pub)
										 .previous (nano::dev::genesis->hash ())
										 .representative (nano::dev::genesis_key.pub) // No representative, blocks can't confirm
										 .balance (nano::dev::constants.genesis_amount / 2 - nano::Gxrb_ratio)
										 .link (key1.pub)
										 .work (0)
										 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
										 .build ();
	node1.work_generate_blocking (*send1);
	ASSERT_EQ (nano::process_result::progress, node1.ledger.process (*node1.store.tx_begin_write (), *send1).code);
	node1.scheduler.activate (nano::dev::genesis_key.pub, *node1.store.tx_begin_read ());
	ASSERT_TIMELY (5s, node1.active.election (send1->qualified_root ()));
	auto election1 = node1.active.election (send1->qualified_root ());
	auto vote1 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 1, 0, std::vector<nano::block_hash>{ send1->hash () }));
	ASSERT_EQ (nano::vote_code::vote, node1.active.vote (vote1));
	// Block is already processed from vote
	ASSERT_TRUE (node1.active.publish (send1));
	ASSERT_EQ (nano::vote::timestamp_min * 1, election1->last_votes[nano::dev::genesis_key.pub].timestamp);
	nano::keypair key2;
	std::shared_ptr<nano::block> send2 = builder.state ()
										 .account (nano::dev::genesis_key.pub)
										 .previous (nano::dev::genesis->hash ())
										 .representative (nano::dev::genesis_key.pub) // No representative, blocks can't confirm
										 .balance (nano::dev::constants.genesis_amount / 2 - nano::Gxrb_ratio)
										 .link (key2.pub)
										 .work (0)
										 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
										 .build ();
	node1.work_generate_blocking (*send2);
	ASSERT_FALSE (node1.active.publish (send2));
	ASSERT_TIMELY (5s, node1.active.active (*send2));
	auto vote2 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 2, 0, std::vector<nano::block_hash>{ send2->hash () }));
	// Pretend we've waited the timeout
	nano::unique_lock<nano::mutex> lock{ election1->mutex };
	election1->last_votes[nano::dev::genesis_key.pub].time = std::chrono::steady_clock::now () - std::chrono::seconds (20);
	lock.unlock ();
	ASSERT_EQ (nano::vote_code::vote, node1.active.vote (vote2));
	ASSERT_EQ (nano::vote::timestamp_min * 2, election1->last_votes[nano::dev::genesis_key.pub].timestamp);
	// Also resend the old vote, and see if we respect the timestamp
	lock.lock ();
	election1->last_votes[nano::dev::genesis_key.pub].time = std::chrono::steady_clock::now () - std::chrono::seconds (20);
	lock.unlock ();
	ASSERT_EQ (nano::vote_code::replay, node1.active.vote (vote1));
	ASSERT_EQ (nano::vote::timestamp_min * 2, election1->votes ()[nano::dev::genesis_key.pub].timestamp);
	auto votes (election1->votes ());
	ASSERT_EQ (2, votes.size ());
	ASSERT_NE (votes.end (), votes.find (nano::dev::genesis_key.pub));
	ASSERT_EQ (send2->hash (), votes[nano::dev::genesis_key.pub].hash);
	ASSERT_EQ (*send2, *election1->tally ().begin ()->second);
}

// Lower timestamps are ignored
TEST (votes, add_old)
{
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	nano::keypair key1;
	nano::block_builder builder;
	auto send1 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key1.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto transaction (node1.store.tx_begin_write ());
	ASSERT_EQ (nano::process_result::progress, node1.ledger.process (*transaction, *send1).code);
	node1.block_confirm (send1);
	ASSERT_TIMELY (5s, node1.active.election (send1->qualified_root ()));
	auto election1 = node1.active.election (send1->qualified_root ());
	auto vote1 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 2, 0, std::vector<nano::block_hash>{ send1->hash () }));
	auto channel (std::make_shared<nano::transport::inproc::channel> (node1, node1));
	node1.vote_processor.vote_blocking (vote1, channel);
	nano::keypair key2;
	auto send2 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key2.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send2);
	auto vote2 = std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 1, 0, std::vector<nano::block_hash>{ send2->hash () });
	{
		nano::lock_guard<nano::mutex> lock{ election1->mutex };
		election1->last_votes[nano::dev::genesis_key.pub].time = std::chrono::steady_clock::now () - std::chrono::seconds (20);
	}
	node1.vote_processor.vote_blocking (vote2, channel);
	ASSERT_EQ (2, election1->votes ().size ());
	auto votes (election1->votes ());
	ASSERT_NE (votes.end (), votes.find (nano::dev::genesis_key.pub));
	ASSERT_EQ (send1->hash (), votes[nano::dev::genesis_key.pub].hash);
	ASSERT_EQ (*send1, *election1->winner ());
}
}

// Lower timestamps are accepted for different accounts
// Test disabled because it's failing intermittently.
// PR in which it got disabled: https://github.com/nanocurrency/nano-node/pull/3629
// Issue for investigating it: https://github.com/nanocurrency/nano-node/issues/3631
TEST (votes, DISABLED_add_old_different_account)
{
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	nano::keypair key1;
	nano::block_builder builder;
	auto send1 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key1.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto send2 = builder
				 .send ()
				 .previous (send1->hash ())
				 .destination (key1.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send2);
	ASSERT_EQ (nano::process_result::progress, node1.process (*send1).code);
	ASSERT_EQ (nano::process_result::progress, node1.process (*send2).code);
	nano::test::blocks_confirm (node1, { send1, send2 });
	auto election1 = node1.active.election (send1->qualified_root ());
	ASSERT_NE (nullptr, election1);
	auto election2 = node1.active.election (send2->qualified_root ());
	ASSERT_NE (nullptr, election2);
	ASSERT_EQ (1, election1->votes ().size ());
	ASSERT_EQ (1, election2->votes ().size ());
	auto vote1 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 2, 0, std::vector<nano::block_hash>{ send1->hash () }));
	auto channel (std::make_shared<nano::transport::inproc::channel> (node1, node1));
	auto vote_result1 (node1.vote_processor.vote_blocking (vote1, channel));
	ASSERT_EQ (nano::vote_code::vote, vote_result1);
	ASSERT_EQ (2, election1->votes ().size ());
	ASSERT_EQ (1, election2->votes ().size ());
	auto vote2 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 1, 0, std::vector<nano::block_hash>{ send2->hash () }));
	auto vote_result2 (node1.vote_processor.vote_blocking (vote2, channel));
	ASSERT_EQ (nano::vote_code::vote, vote_result2);
	ASSERT_EQ (2, election1->votes ().size ());
	ASSERT_EQ (2, election2->votes ().size ());
	auto votes1 (election1->votes ());
	auto votes2 (election2->votes ());
	ASSERT_NE (votes1.end (), votes1.find (nano::dev::genesis_key.pub));
	ASSERT_NE (votes2.end (), votes2.find (nano::dev::genesis_key.pub));
	ASSERT_EQ (send1->hash (), votes1[nano::dev::genesis_key.pub].hash);
	ASSERT_EQ (send2->hash (), votes2[nano::dev::genesis_key.pub].hash);
	ASSERT_EQ (*send1, *election1->winner ());
	ASSERT_EQ (*send2, *election2->winner ());
}

// The voting cooldown is respected
TEST (votes, add_cooldown)
{
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	nano::keypair key1;
	nano::block_builder builder;
	auto send1 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key1.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto transaction (node1.store.tx_begin_write ());
	ASSERT_EQ (nano::process_result::progress, node1.ledger.process (*transaction, *send1).code);
	node1.block_confirm (send1);
	ASSERT_TIMELY (5s, node1.active.election (send1->qualified_root ()));
	auto election1 = node1.active.election (send1->qualified_root ());
	auto vote1 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 1, 0, std::vector<nano::block_hash>{ send1->hash () }));
	auto channel (std::make_shared<nano::transport::inproc::channel> (node1, node1));
	node1.vote_processor.vote_blocking (vote1, channel);
	nano::keypair key2;
	auto send2 = builder
				 .send ()
				 .previous (nano::dev::genesis->hash ())
				 .destination (key2.pub)
				 .balance (0)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send2);
	auto vote2 (std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, nano::vote::timestamp_min * 2, 0, std::vector<nano::block_hash>{ send2->hash () }));
	node1.vote_processor.vote_blocking (vote2, channel);
	ASSERT_EQ (2, election1->votes ().size ());
	auto votes (election1->votes ());
	ASSERT_NE (votes.end (), votes.find (nano::dev::genesis_key.pub));
	ASSERT_EQ (send1->hash (), votes[nano::dev::genesis_key.pub].hash);
	ASSERT_EQ (*send1, *election1->winner ());
}

TEST (ledger, epoch_open_pending)
{
	nano::block_builder builder{};
	nano::test::system system{ 1 };
	auto & node1 = *system.nodes[0];
	nano::work_pool pool{ nano::dev::network_params.network, std::numeric_limits<unsigned>::max () };
	nano::keypair key1{};
	auto epoch_open = builder.state ()
					  .account (key1.pub)
					  .previous (0)
					  .representative (0)
					  .balance (0)
					  .link (node1.ledger.epoch_link (nano::epoch::epoch_1))
					  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					  .work (*pool.generate (key1.pub))
					  .build_shared ();
	auto process_result = node1.ledger.process (*node1.store.tx_begin_write (), *epoch_open);
	ASSERT_EQ (nano::process_result::gap_epoch_open_pending, process_result.code);
	node1.block_processor.add (epoch_open);
	// Waits for the block to get saved in the database
	ASSERT_TIMELY (10s, 1 == node1.unchecked.count (*node1.store.tx_begin_read ()));
	ASSERT_FALSE (node1.ledger.block_or_pruned_exists (epoch_open->hash ()));
	// Open block should be inserted into unchecked
	auto blocks = node1.unchecked.get (*node1.store.tx_begin_read (), nano::hash_or_account (epoch_open->account ()).hash);
	ASSERT_EQ (blocks.size (), 1);
	ASSERT_EQ (blocks[0].get_block ()->full_hash (), epoch_open->full_hash ());
	// New block to process epoch open
	auto send1 = builder.state ()
				 .account (nano::dev::genesis->account ())
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (nano::dev::constants.genesis_amount - 100)
				 .link (key1.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*pool.generate (nano::dev::genesis->hash ()))
				 .build_shared ();
	node1.block_processor.add (send1);
	ASSERT_TIMELY (10s, node1.ledger.block_or_pruned_exists (epoch_open->hash ()));
}

TEST (ledger, block_hash_account_conflict)
{
	nano::block_builder builder;
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	nano::keypair key1;
	nano::keypair key2;
	nano::work_pool pool{ nano::dev::network_params.network, std::numeric_limits<unsigned>::max () };

	/*
	 * Generate a send block whose destination is a block hash already
	 * in the ledger and not an account
	 */
	auto send1 = builder.state ()
				 .account (nano::dev::genesis->account ())
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (nano::dev::constants.genesis_amount - 100)
				 .link (key1.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*pool.generate (nano::dev::genesis->hash ()))
				 .build_shared ();

	auto receive1 = builder.state ()
					.account (key1.pub)
					.previous (0)
					.representative (nano::dev::genesis->account ())
					.balance (100)
					.link (send1->hash ())
					.sign (key1.prv, key1.pub)
					.work (*pool.generate (key1.pub))
					.build_shared ();

	/*
	 * Note that the below link is a block hash when this is intended
	 * to represent a send state block. This can generally never be
	 * received , except by epoch blocks, which can sign an open block
	 * for arbitrary accounts.
	 */
	auto send2 = builder.state ()
				 .account (key1.pub)
				 .previous (receive1->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (90)
				 .link (receive1->hash ())
				 .sign (key1.prv, key1.pub)
				 .work (*pool.generate (receive1->hash ()))
				 .build_shared ();

	/*
	 * Generate an epoch open for the account with the same value as the block hash
	 */
	auto receive1_hash = receive1->hash ();
	auto open_epoch1 = builder.state ()
					   .account (reinterpret_cast<nano::account const &> (receive1_hash))
					   .previous (0)
					   .representative (0)
					   .balance (0)
					   .link (node1.ledger.epoch_link (nano::epoch::epoch_1))
					   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					   .work (*pool.generate (receive1->hash ()))
					   .build_shared ();

	node1.work_generate_blocking (*send1);
	node1.work_generate_blocking (*receive1);
	node1.work_generate_blocking (*send2);
	node1.work_generate_blocking (*open_epoch1);
	ASSERT_EQ (nano::process_result::progress, node1.process (*send1).code);
	ASSERT_EQ (nano::process_result::progress, node1.process (*receive1).code);
	ASSERT_EQ (nano::process_result::progress, node1.process (*send2).code);
	ASSERT_EQ (nano::process_result::progress, node1.process (*open_epoch1).code);
	nano::test::blocks_confirm (node1, { send1, receive1, send2, open_epoch1 });
	auto election1 = node1.active.election (send1->qualified_root ());
	ASSERT_NE (nullptr, election1);
	auto election2 = node1.active.election (receive1->qualified_root ());
	ASSERT_NE (nullptr, election2);
	auto election3 = node1.active.election (send2->qualified_root ());
	ASSERT_NE (nullptr, election3);
	auto election4 = node1.active.election (open_epoch1->qualified_root ());
	ASSERT_NE (nullptr, election4);
	auto winner1 (election1->winner ());
	auto winner2 (election2->winner ());
	auto winner3 (election3->winner ());
	auto winner4 (election4->winner ());
	ASSERT_EQ (*send1, *winner1);
	ASSERT_EQ (*receive1, *winner2);
	ASSERT_EQ (*send2, *winner3);
	ASSERT_EQ (*open_epoch1, *winner4);
}

TEST (ledger, unchecked_epoch)
{
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	nano::keypair destination;
	nano::block_builder builder;
	auto send1 = builder
				 .state ()
				 .account (nano::dev::genesis->account ())
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (destination.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto open1 = builder
				 .state ()
				 .account (destination.pub)
				 .previous (0)
				 .representative (destination.pub)
				 .balance (nano::Gxrb_ratio)
				 .link (send1->hash ())
				 .sign (destination.prv, destination.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*open1);
	auto epoch1 = builder
				  .state ()
				  .account (destination.pub)
				  .previous (open1->hash ())
				  .representative (destination.pub)
				  .balance (nano::Gxrb_ratio)
				  .link (node1.ledger.epoch_link (nano::epoch::epoch_1))
				  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				  .work (0)
				  .build_shared ();
	node1.work_generate_blocking (*epoch1);
	node1.block_processor.add (epoch1);
	{
		// Waits for the epoch1 block to pass through block_processor and unchecked.put queues
		ASSERT_TIMELY (10s, 1 == node1.unchecked.count (*node1.store.tx_begin_read ()));
		auto blocks = node1.unchecked.get (*node1.store.tx_begin_read (), epoch1->previous ());
		ASSERT_EQ (blocks.size (), 1);
	}
	node1.block_processor.add (send1);
	node1.block_processor.add (open1);
	ASSERT_TIMELY (5s, node1.store.block ().exists (*node1.store.tx_begin_read (), epoch1->hash ()));
	{
		// Waits for the last blocks to pass through block_processor and unchecked.put queues
		ASSERT_TIMELY (10s, 0 == node1.unchecked.count (*node1.store.tx_begin_read ()));
		nano::account_info info{};
		ASSERT_FALSE (node1.store.account ().get (*node1.store.tx_begin_read (), destination.pub, info));
		ASSERT_EQ (info.epoch (), nano::epoch::epoch_1);
	}
}

TEST (ledger, unchecked_epoch_invalid)
{
	nano::test::system system;
	nano::node_config node_config (nano::test::get_available_port (), system.logging);
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto & node1 (*system.add_node (node_config));
	nano::keypair destination;
	nano::block_builder builder;
	auto send1 = builder
				 .state ()
				 .account (nano::dev::genesis->account ())
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (destination.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto open1 = builder
				 .state ()
				 .account (destination.pub)
				 .previous (0)
				 .representative (destination.pub)
				 .balance (nano::Gxrb_ratio)
				 .link (send1->hash ())
				 .sign (destination.prv, destination.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*open1);
	// Epoch block with account own signature
	auto epoch1 = builder
				  .state ()
				  .account (destination.pub)
				  .previous (open1->hash ())
				  .representative (destination.pub)
				  .balance (nano::Gxrb_ratio)
				  .link (node1.ledger.epoch_link (nano::epoch::epoch_1))
				  .sign (destination.prv, destination.pub)
				  .work (0)
				  .build_shared ();
	node1.work_generate_blocking (*epoch1);
	// Pseudo epoch block (send subtype, destination - epoch link)
	auto epoch2 = builder
				  .state ()
				  .account (destination.pub)
				  .previous (open1->hash ())
				  .representative (destination.pub)
				  .balance (nano::Gxrb_ratio - 1)
				  .link (node1.ledger.epoch_link (nano::epoch::epoch_1))
				  .sign (destination.prv, destination.pub)
				  .work (0)
				  .build_shared ();
	node1.work_generate_blocking (*epoch2);
	node1.block_processor.add (epoch1);
	node1.block_processor.add (epoch2);
	{
		// Waits for the last blocks to pass through block_processor and unchecked.put queues
		ASSERT_TIMELY (10s, 2 == node1.unchecked.count (*node1.store.tx_begin_read ()));
		auto blocks = node1.unchecked.get (*node1.store.tx_begin_read (), epoch1->previous ());
		ASSERT_EQ (blocks.size (), 2);
	}
	node1.block_processor.add (send1);
	node1.block_processor.add (open1);
	// Waits for the last blocks to pass through block_processor and unchecked.put queues
	ASSERT_TIMELY (10s, node1.store.block ().exists (*node1.store.tx_begin_read (), epoch2->hash ()));
	{
		auto transaction = node1.store.tx_begin_read ();
		ASSERT_FALSE (node1.store.block ().exists (*transaction, epoch1->hash ()));
		auto unchecked_count = node1.unchecked.count (*transaction);
		ASSERT_EQ (unchecked_count, 0);
		ASSERT_EQ (unchecked_count, node1.unchecked.count (*transaction));
		nano::account_info info{};
		ASSERT_FALSE (node1.store.account ().get (*transaction, destination.pub, info));
		ASSERT_NE (info.epoch (), nano::epoch::epoch_1);
		auto epoch2_store = node1.store.block ().get (*transaction, epoch2->hash ());
		ASSERT_NE (nullptr, epoch2_store);
		ASSERT_EQ (nano::epoch::epoch_0, epoch2_store->sideband ().details ().epoch ());
		ASSERT_TRUE (epoch2_store->sideband ().details ().is_send ());
		ASSERT_FALSE (epoch2_store->sideband ().details ().is_epoch ());
		ASSERT_FALSE (epoch2_store->sideband ().details ().is_receive ());
	}
}

TEST (ledger, unchecked_open)
{
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	nano::keypair destination;
	nano::block_builder builder;
	auto send1 = builder
				 .state ()
				 .account (nano::dev::genesis->account ())
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (destination.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto open1 = builder
				 .open ()
				 .source (send1->hash ())
				 .representative (destination.pub)
				 .account (destination.pub)
				 .sign (destination.prv, destination.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*open1);
	// Invalid signature for open block
	auto open2 = builder
				 .open ()
				 .source (send1->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .account (destination.pub)
				 .sign (destination.prv, destination.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*open2);
	auto sig{ open2->block_signature () };
	sig.bytes[0] ^= 1;
	open2->signature_set (sig);
	node1.block_processor.add (open1);
	node1.block_processor.add (open2);
	{
		// Waits for the last blocks to pass through block_processor and unchecked.put queues
		ASSERT_TIMELY (10s, 1 == node1.unchecked.count (*node1.store.tx_begin_read ()));
		auto blocks = node1.unchecked.get (*node1.store.tx_begin_read (), open1->source ());
		ASSERT_EQ (blocks.size (), 1);
	}
	node1.block_processor.add (send1);
	// Waits for the send1 block to pass through block_processor and unchecked.put queues
	ASSERT_TIMELY (10s, node1.store.block ().exists (*node1.store.tx_begin_read (), open1->hash ()));
	ASSERT_EQ (0, node1.unchecked.count (*node1.store.tx_begin_read ()));
}

TEST (ledger, unchecked_receive)
{
	nano::test::system system{ 1 };
	auto & node1 = *system.nodes[0];
	nano::keypair destination{};
	nano::block_builder builder;
	auto send1 = builder
				 .state ()
				 .account (nano::dev::genesis->account ())
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (destination.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send1);
	auto send2 = builder
				 .state ()
				 .account (nano::dev::genesis->account ())
				 .previous (send1->hash ())
				 .representative (nano::dev::genesis->account ())
				 .balance (nano::dev::constants.genesis_amount - 2 * nano::Gxrb_ratio)
				 .link (destination.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*send2);
	auto open1 = builder
				 .open ()
				 .source (send1->hash ())
				 .representative (destination.pub)
				 .account (destination.pub)
				 .sign (destination.prv, destination.pub)
				 .work (0)
				 .build_shared ();
	node1.work_generate_blocking (*open1);
	auto receive1 = builder
					.receive ()
					.previous (open1->hash ())
					.source (send2->hash ())
					.sign (destination.prv, destination.pub)
					.work (0)
					.build_shared ();
	node1.work_generate_blocking (*receive1);
	node1.block_processor.add (send1);
	node1.block_processor.add (receive1);
	auto check_block_is_listed = [&] (nano::transaction const & transaction_a, nano::block_hash const & block_hash_a) {
		return !node1.unchecked.get (transaction_a, block_hash_a).empty ();
	};
	// Previous block for receive1 is unknown, signature cannot be validated
	{
		// Waits for the last blocks to pass through block_processor and unchecked.put queues
		ASSERT_TIMELY (15s, check_block_is_listed (*node1.store.tx_begin_read (), receive1->previous ()));
		auto blocks = node1.unchecked.get (*node1.store.tx_begin_read (), receive1->previous ());
		ASSERT_EQ (blocks.size (), 1);
	}
	// Waits for the open1 block to pass through block_processor and unchecked.put queues
	node1.block_processor.add (open1);
	ASSERT_TIMELY (15s, check_block_is_listed (*node1.store.tx_begin_read (), receive1->source ()));
	// Previous block for receive1 is known, signature was validated
	{
		auto transaction = node1.store.tx_begin_read ();
		auto blocks (node1.unchecked.get (*transaction, receive1->source ()));
		ASSERT_EQ (blocks.size (), 1);
	}
	node1.block_processor.add (send2);
	ASSERT_TIMELY (10s, node1.store.block ().exists (*node1.store.tx_begin_read (), receive1->hash ()));
	ASSERT_EQ (0, node1.unchecked.count (*node1.store.tx_begin_read ()));
}
