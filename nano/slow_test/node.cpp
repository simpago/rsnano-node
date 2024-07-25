#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/lib/logging.hpp>
#include <nano/lib/thread_runner.hpp>
#include <nano/lib/threading.hpp>
#include <nano/node/election.hpp>
#include <nano/node/make_store.hpp>
#include <nano/node/scheduler/component.hpp>
#include <nano/node/scheduler/manual.hpp>
#include <nano/node/scheduler/priority.hpp>
#include <nano/node/transport/inproc.hpp>
#include <nano/node/unchecked_map.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/test_common/network.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <boost/format.hpp>
#include <boost/unordered_set.hpp>

using namespace std::chrono_literals;

/**
 * function to count the block in the pruned store one by one
 * we manually count the blocks one by one because the rocksdb count feature is not accurate
 */
size_t manually_count_pruned_blocks (nano::store::component & store)
{
	size_t count = 0;
	auto transaction = store.tx_begin_read ();
	auto i = store.pruned ().begin (*transaction);
	for (; i != store.pruned ().end (); ++i)
	{
		++count;
	}
	return count;
}

TEST (system, generate_mass_activity)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.enable_voting = false; // Prevent blocks cementing
	auto node = system.add_node (node_config);
	(void)node->wallets.insert_adhoc (node->wallets.first_wallet_id (), nano::dev::genesis_key.prv);
	uint32_t count (20);
	system.generate_mass_activity (count, *system.nodes[0]);
	auto transaction (system.nodes[0]->store.tx_begin_read ());
	for (auto i (system.nodes[0]->store.account ().begin (*transaction)), n (system.nodes[0]->store.account ().end ()); i != n; ++i)
	{
	}
}

TEST (system, generate_mass_activity_long)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.enable_voting = false; // Prevent blocks cementing
	auto node = system.add_node (node_config);
	nano::thread_runner runner (system.async_rt.io_ctx, system.nodes[0]->config->io_threads);
	(void)node->wallets.insert_adhoc (node->wallets.first_wallet_id (), nano::dev::genesis_key.prv);
	uint32_t count (1000000);
	auto count_env_var = std::getenv ("SLOW_TEST_SYSTEM_GENERATE_MASS_ACTIVITY_LONG_COUNT");
	if (count_env_var)
	{
		count = boost::lexical_cast<uint32_t> (count_env_var);
		std::cout << "count override due to env variable set, count=" << count << std::endl;
	}
	system.generate_mass_activity (count, *system.nodes[0]);
	auto transaction (system.nodes[0]->store.tx_begin_read ());
	for (auto i (system.nodes[0]->store.account ().begin (*transaction)), n (system.nodes[0]->store.account ().end ()); i != n; ++i)
	{
	}
	system.stop ();
	runner.join ();
}

TEST (system, receive_while_synchronizing)
{
	std::vector<boost::thread> threads;
	{
		nano::test::system system;
		nano::node_config node_config = system.default_config ();
		node_config.enable_voting = false; // Prevent blocks cementing
		auto node = system.add_node (node_config);
		auto wallet_id = node->wallets.first_wallet_id ();

		nano::thread_runner runner (system.async_rt.io_ctx, system.nodes[0]->config->io_threads);
		(void)node->wallets.insert_adhoc (node->wallets.first_wallet_id (), nano::dev::genesis_key.prv);
		uint32_t count (1000);
		system.generate_mass_activity (count, *system.nodes[0]);
		nano::keypair key;
		auto node1 (std::make_shared<nano::node> (system.async_rt, system.get_available_port (), nano::unique_path (), system.work));
		ASSERT_FALSE (node1->init_error ());
		node1->wallets.create (1);
		nano::account account;
		ASSERT_EQ (nano::wallets_error::none, node1->wallets.insert_adhoc (1, nano::dev::genesis_key.prv, true, account)); // For voting
		ASSERT_EQ (nano::wallets_error::none, node1->wallets.insert_adhoc (1, key.prv, true, account));
		ASSERT_EQ (key.pub, account);
		node1->start ();
		system.nodes.push_back (node1);
		ASSERT_NE (nullptr, nano::test::establish_tcp (system, *node1, node->network->endpoint ()));
		node1->workers->add_timed_task (std::chrono::steady_clock::now () + std::chrono::milliseconds (200), ([&system, &key, &node, &wallet_id] () {
			auto hash (node->wallets.send_sync (wallet_id, nano::dev::genesis_key.pub, key.pub, system.nodes[0]->config->receive_minimum.number ()));
			auto transaction (system.nodes[0]->store.tx_begin_read ());
			auto block (system.nodes[0]->ledger.any ().block_get (*transaction, hash));
			std::string block_text;
			block->serialize_json (block_text);
		}));
		ASSERT_TIMELY (10s, !node1->balance (key.pub).is_zero ());
		node1->stop ();
		system.stop ();
		runner.join ();
	}
	for (auto i (threads.begin ()), n (threads.end ()); i != n; ++i)
	{
		i->join ();
	}
}

/*
 * This test case creates a node and a wallet primed with the genesis account credentials.
 * Then it spawns 'num_of_threads' threads, each doing 'num_of_sends' async sends
 * of 1000 raw each time. The test is considered a success, if the balance of the genesis account
 * reduces by 'num_of_threads * num_of_sends * 1000'.
 */
TEST (wallet, multithreaded_send_async)
{
	std::vector<boost::thread> threads;
	{
		nano::test::system system (1);
		nano::keypair key;
		auto node = system.nodes[0];
		auto wallet_id = node->wallets.first_wallet_id ();
		(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
		(void)node->wallets.insert_adhoc (wallet_id, key.prv);
		int num_of_threads = 20;
		int num_of_sends = 1000;
		for (auto i (0); i < num_of_threads; ++i)
		{
			threads.push_back (boost::thread ([&key, num_of_threads, num_of_sends, node, wallet_id] () {
				for (auto i (0); i < num_of_sends; ++i)
				{
					(void)node->wallets.send_async (wallet_id, nano::dev::genesis_key.pub, key.pub, 1000, [] (std::shared_ptr<nano::block> const & block_a) {
						ASSERT_FALSE (block_a == nullptr);
						ASSERT_FALSE (block_a->hash ().is_zero ());
					});
				}
			}));
		}
		ASSERT_TIMELY_EQ (1000s, system.nodes[0]->balance (nano::dev::genesis_key.pub), (nano::dev::constants.genesis_amount - num_of_threads * num_of_sends * 1000));
	}
	for (auto i (threads.begin ()), n (threads.end ()); i != n; ++i)
	{
		i->join ();
	}
}

TEST (store, load)
{
	nano::test::system system (1);
	std::vector<boost::thread> threads;
	for (auto i (0); i < 100; ++i)
	{
		threads.push_back (boost::thread ([&system] () {
			for (auto i (0); i != 1000; ++i)
			{
				auto transaction (system.nodes[0]->store.tx_begin_write ());
				for (auto j (0); j != 10; ++j)
				{
					nano::account account;
					nano::random_pool::generate_block (account.bytes.data (), account.bytes.size ());
					system.nodes[0]->store.account ().put (*transaction, account, nano::account_info ());
				}
			}
		}));
	}
	for (auto & i : threads)
	{
		i.join ();
	}
}

namespace
{
size_t heard_count (std::vector<uint8_t> const & nodes)
{
	auto result (0);
	for (auto i (nodes.begin ()), n (nodes.end ()); i != n; ++i)
	{
		switch (*i)
		{
			case 0:
				break;
			case 1:
				++result;
				break;
			case 2:
				++result;
				break;
		}
	}
	return result;
}
}

TEST (broadcast, world_broadcast_simulate)
{
	auto node_count (10000);
	// 0 = starting state
	// 1 = heard transaction
	// 2 = repeated transaction
	std::vector<uint8_t> nodes;
	nodes.resize (node_count, 0);
	nodes[0] = 1;
	auto any_changed (true);
	auto message_count (0);
	while (any_changed)
	{
		any_changed = false;
		for (auto i (nodes.begin ()), n (nodes.end ()); i != n; ++i)
		{
			switch (*i)
			{
				case 0:
					break;
				case 1:
					for (auto j (nodes.begin ()), m (nodes.end ()); j != m; ++j)
					{
						++message_count;
						switch (*j)
						{
							case 0:
								*j = 1;
								any_changed = true;
								break;
							case 1:
								break;
							case 2:
								break;
						}
					}
					*i = 2;
					any_changed = true;
					break;
				case 2:
					break;
				default:
					ASSERT_FALSE (true);
					break;
			}
		}
	}
	auto count (heard_count (nodes));
	(void)count;
}

TEST (broadcast, sqrt_broadcast_simulate)
{
	auto node_count (10000);
	auto broadcast_count (std::ceil (std::sqrt (node_count)));
	// 0 = starting state
	// 1 = heard transaction
	// 2 = repeated transaction
	std::vector<uint8_t> nodes;
	nodes.resize (node_count, 0);
	nodes[0] = 1;
	auto any_changed (true);
	uint64_t message_count (0);
	while (any_changed)
	{
		any_changed = false;
		for (auto i (nodes.begin ()), n (nodes.end ()); i != n; ++i)
		{
			switch (*i)
			{
				case 0:
					break;
				case 1:
					for (auto j (0); j != broadcast_count; ++j)
					{
						++message_count;
						auto entry (nano::random_pool::generate_word32 (0, node_count - 1));
						switch (nodes[entry])
						{
							case 0:
								nodes[entry] = 1;
								any_changed = true;
								break;
							case 1:
								break;
							case 2:
								break;
						}
					}
					*i = 2;
					any_changed = true;
					break;
				case 2:
					break;
				default:
					ASSERT_FALSE (true);
					break;
			}
		}
	}
	auto count (heard_count (nodes));
	(void)count;
}

// Can take up to 2 hours
TEST (store, unchecked_load)
{
	nano::test::system system{ 1 };
	auto & node = *system.nodes[0];
	nano::block_builder builder;
	std::shared_ptr<nano::block> block = builder
										 .send ()
										 .previous (0)
										 .destination (0)
										 .balance (0)
										 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
										 .work (0)
										 .build ();
	constexpr auto num_unchecked = 1'000'000;
	for (auto i (0); i < num_unchecked; ++i)
	{
		node.unchecked.put (i, block);
	}
	// Waits for all the blocks to get saved in the database
	ASSERT_TIMELY_EQ (8000s, num_unchecked, node.unchecked.count ());
}

TEST (store, vote_load)
{
	nano::test::system system{ 1 };
	auto & node = *system.nodes[0];
	for (auto i = 0u; i < 1000000u; ++i)
	{
		auto vote = std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, i, 0, std::vector<nano::block_hash>{ i });
		node.vote_processor_queue.vote (vote, std::make_shared<nano::transport::inproc::channel> (node, node));
	}
}

/**
 * This test does the following:
 *   Creates a persistent database in the file system
 *   Adds 2 million random blocks to the database in chunks of 20 blocks per database transaction
 *   It then deletes half the blocks, soon after adding them
 *   Then it closes the database, reopens the database and checks that it still has the expected amount of blocks
 */
TEST (store, pruned_load)
{
	auto path (nano::unique_path ());
	constexpr auto num_pruned = 2000000;
	auto const expected_result = num_pruned / 2;
	constexpr auto batch_size = 20;
	boost::unordered_set<nano::block_hash> hashes;
	{
		auto store = nano::make_store (path, nano::dev::constants);
		ASSERT_FALSE (store->init_error ());
		for (auto i (0); i < num_pruned / batch_size; ++i)
		{
			{
				// write a batch of random blocks to the pruned store
				auto transaction (store->tx_begin_write ());
				for (auto k (0); k < batch_size; ++k)
				{
					nano::block_hash random_hash;
					nano::random_pool::generate_block (random_hash.bytes.data (), random_hash.bytes.size ());
					store->pruned ().put (*transaction, random_hash);
					hashes.insert (random_hash);
				}
			}
			{
				// delete half of the blocks created above
				auto transaction (store->tx_begin_write ());
				for (auto k (0); !hashes.empty () && k < batch_size / 2; ++k)
				{
					auto hash (hashes.begin ());
					store->pruned ().del (*transaction, *hash);
					hashes.erase (hash);
				}
			}
		}
		ASSERT_EQ (expected_result, manually_count_pruned_blocks (*store));
	}

	// Reinitialize store
	{
		auto store = nano::make_store (path, nano::dev::constants);
		ASSERT_FALSE (store->init_error ());
		ASSERT_EQ (expected_result, manually_count_pruned_blocks (*store));
	}
}

TEST (wallets, rep_scan)
{
	nano::test::system system (1);
	auto & node (*system.nodes[0]);
	auto wallet_id = node.wallets.first_wallet_id ();
	{
		for (auto i (0); i < 10000; ++i)
		{
			nano::public_key account;
			(void)node.wallets.deterministic_insert (wallet_id, true, account);
		}
	}
	auto begin (std::chrono::steady_clock::now ());
	node.wallets.foreach_representative ([] (nano::public_key const & pub_a, nano::raw_key const & prv_a) {
	});
	ASSERT_LT (std::chrono::steady_clock::now () - begin, std::chrono::milliseconds (5));
}

TEST (node, mass_vote_by_hash)
{
	nano::test::system system (1);
	auto node = system.nodes[0];
	auto wallet_id = node->wallets.first_wallet_id ();
	(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	nano::block_hash previous (nano::dev::genesis->hash ());
	nano::keypair key;
	std::vector<std::shared_ptr<nano::state_block>> blocks;
	nano::block_builder builder;
	for (auto i (0); i < 10000; ++i)
	{
		auto block = builder
					 .state ()
					 .account (nano::dev::genesis_key.pub)
					 .previous (previous)
					 .representative (nano::dev::genesis_key.pub)
					 .balance (nano::dev::constants.genesis_amount - (i + 1) * nano::Gxrb_ratio)
					 .link (key.pub)
					 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					 .work (*system.work.generate (previous))
					 .build ();
		previous = block->hash ();
		blocks.push_back (block);
	}
	for (auto i (blocks.begin ()), n (blocks.end ()); i != n; ++i)
	{
		system.nodes[0]->block_processor.add (*i);
	}
}

namespace nano
{
TEST (confirmation_height, many_accounts_single_confirmation)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.online_weight_minimum = 100;
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto node = system.add_node (node_config);
	auto wallet_id = node->wallets.first_wallet_id ();
	(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);

	// The number of frontiers should be more than the nano::confirmation_height::unbounded_cutoff to test the amount of blocks confirmed is correct.
	auto const num_accounts = nano::confirmation_height::unbounded_cutoff * 2 + 50;
	nano::keypair last_keypair = nano::dev::genesis_key;
	nano::block_builder builder;
	auto last_open_hash = node->latest (nano::dev::genesis_key.pub);
	{
		auto transaction = node->store.tx_begin_write ();
		for (auto i = num_accounts - 1; i > 0; --i)
		{
			nano::keypair key;
			(void)node->wallets.insert_adhoc (wallet_id, key.prv);

			auto send = builder
						.send ()
						.previous (last_open_hash)
						.destination (key.pub)
						.balance (node->quorum ().quorum_delta)
						.sign (last_keypair.prv, last_keypair.pub)
						.work (*system.work.generate (last_open_hash))
						.build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send));
			auto open = builder
						.open ()
						.source (send->hash ())
						.representative (last_keypair.pub)
						.account (key.pub)
						.sign (key.prv, key.pub)
						.work (*system.work.generate (key.pub))
						.build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, open));
			last_open_hash = open->hash ();
			last_keypair = key;
		}
	}

	// Call block confirm on the last open block which will confirm everything
	{
		auto block = node->block (last_open_hash);
		ASSERT_NE (nullptr, block);
		node->scheduler.manual.push (block);
		std::shared_ptr<nano::election> election;
		ASSERT_TIMELY (10s, (election = node->active.election (block->qualified_root ())) != nullptr);
		node->active.force_confirm (*election);
	}

	ASSERT_TIMELY (120s, node->ledger.confirmed ().block_exists (*node->store.tx_begin_read (), last_open_hash));

	// All frontiers (except last) should have 2 blocks and both should be confirmed
	auto transaction = node->store.tx_begin_read ();
	for (auto i (node->store.account ().begin (*transaction)), n (node->store.account ().end ()); i != n; ++i)
	{
		auto & account = i->first;
		auto & account_info = i->second;
		auto count = (account != last_keypair.pub) ? 2 : 1;
		nano::confirmation_height_info confirmation_height_info;
		ASSERT_FALSE (node->store.confirmation_height ().get (*transaction, account, confirmation_height_info));
		ASSERT_EQ (count, confirmation_height_info.height ());
		ASSERT_EQ (count, account_info.block_count ());
	}

	size_t cemented_count = 0;
	for (auto i (node->ledger.store.confirmation_height ().begin (*transaction)), n (node->ledger.store.confirmation_height ().end ()); i != n; ++i)
	{
		cemented_count += i->second.height ();
	}

	ASSERT_EQ (cemented_count, node->ledger.cemented_count ());
	ASSERT_EQ (node->stats->count (nano::stat::type::confirmation_height, nano::stat::detail::blocks_confirmed, nano::stat::dir::in), num_accounts * 2 - 2);

	ASSERT_TIMELY_EQ (40s, (node->ledger.cemented_count () - 1), node->stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::all, nano::stat::dir::out));
	ASSERT_TIMELY_EQ (10s, node->active.election_winner_details_size (), 0);
}

TEST (confirmation_height, many_accounts_many_confirmations)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.online_weight_minimum = 100;
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto node = system.add_node (node_config);
	auto wallet_id = node->wallets.first_wallet_id ();
	(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);

	auto const num_accounts = nano::confirmation_height::unbounded_cutoff * 2 + 50;
	auto latest_genesis = node->latest (nano::dev::genesis_key.pub);
	nano::block_builder builder;
	std::vector<std::shared_ptr<nano::open_block>> open_blocks;
	{
		auto transaction = node->store.tx_begin_write ();
		for (auto i = num_accounts - 1; i > 0; --i)
		{
			nano::keypair key;
			(void)node->wallets.insert_adhoc (wallet_id, key.prv);

			auto send = builder
						.send ()
						.previous (latest_genesis)
						.destination (key.pub)
						.balance (node->quorum ().quorum_delta)
						.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
						.work (*system.work.generate (latest_genesis))
						.build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send));
			auto open = builder
						.open ()
						.source (send->hash ())
						.representative (nano::dev::genesis_key.pub)
						.account (key.pub)
						.sign (key.prv, key.pub)
						.work (*system.work.generate (key.pub))
						.build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, open));
			open_blocks.push_back (std::move (open));
			latest_genesis = send->hash ();
		}
	}

	// Confirm all of the accounts
	for (auto & open_block : open_blocks)
	{
		node->scheduler.manual.push (open_block);
		std::shared_ptr<nano::election> election;
		ASSERT_TIMELY (10s, (election = node->active.election (open_block->qualified_root ())) != nullptr);
		node->active.force_confirm (*election);
	}

	auto const num_blocks_to_confirm = (num_accounts - 1) * 2;
	ASSERT_TIMELY_EQ (1500s, node->stats->count (nano::stat::type::confirmation_height, nano::stat::detail::blocks_confirmed, nano::stat::dir::in), num_blocks_to_confirm);

	ASSERT_TIMELY_EQ (60s, (node->ledger.cemented_count () - 1), node->stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::all, nano::stat::dir::out));

	auto transaction = node->store.tx_begin_read ();
	size_t cemented_count = 0;
	for (auto i (node->ledger.store.confirmation_height ().begin (*transaction)), n (node->ledger.store.confirmation_height ().end ()); i != n; ++i)
	{
		cemented_count += i->second.height ();
	}

	ASSERT_EQ (num_blocks_to_confirm + 1, cemented_count);
	ASSERT_EQ (cemented_count, node->ledger.cemented_count ());

	ASSERT_TIMELY_EQ (20s, (node->ledger.cemented_count () - 1), node->stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::all, nano::stat::dir::out));

	ASSERT_TIMELY_EQ (10s, node->active.election_winner_details_size (), 0);
}

TEST (confirmation_height, long_chains)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto node = system.add_node (node_config);
	auto wallet_id = node->wallets.first_wallet_id ();
	nano::keypair key1;
	(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	nano::block_hash latest (node->latest (nano::dev::genesis_key.pub));
	(void)node->wallets.insert_adhoc (wallet_id, key1.prv);

	auto const num_blocks = nano::confirmation_height::unbounded_cutoff * 2 + 50;

	nano::block_builder builder;
	// First open the other account
	auto send = builder
				.send ()
				.previous (latest)
				.destination (key1.pub)
				.balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio + num_blocks + 1)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (latest))
				.build ();
	auto open = builder
				.open ()
				.source (send->hash ())
				.representative (nano::dev::genesis_key.pub)
				.account (key1.pub)
				.sign (key1.prv, key1.pub)
				.work (*system.work.generate (key1.pub))
				.build ();
	{
		auto transaction = node->store.tx_begin_write ();
		ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send));
		ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, open));
	}

	// Bulk send from genesis account to destination account
	auto previous_genesis_chain_hash = send->hash ();
	auto previous_destination_chain_hash = open->hash ();
	{
		auto transaction = node->store.tx_begin_write ();
		for (auto i = num_blocks - 1; i > 0; --i)
		{
			auto send = builder
						.send ()
						.previous (previous_genesis_chain_hash)
						.destination (key1.pub)
						.balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio + i + 1)
						.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
						.work (*system.work.generate (previous_genesis_chain_hash))
						.build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send));
			auto receive = builder
						   .receive ()
						   .previous (previous_destination_chain_hash)
						   .source (send->hash ())
						   .sign (key1.prv, key1.pub)
						   .work (*system.work.generate (previous_destination_chain_hash))
						   .build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, receive));

			previous_genesis_chain_hash = send->hash ();
			previous_destination_chain_hash = receive->hash ();
		}
	}

	// Send one from destination to genesis and pocket it
	auto send1 = builder
				 .send ()
				 .previous (previous_destination_chain_hash)
				 .destination (nano::dev::genesis_key.pub)
				 .balance (nano::Gxrb_ratio - 2)
				 .sign (key1.prv, key1.pub)
				 .work (*system.work.generate (previous_destination_chain_hash))
				 .build ();
	auto receive1 = builder
					.state ()
					.account (nano::dev::genesis_key.pub)
					.previous (previous_genesis_chain_hash)
					.representative (nano::dev::genesis_key.pub)
					.balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio + 1)
					.link (send1->hash ())
					.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					.work (*system.work.generate (previous_genesis_chain_hash))
					.build ();

	// Unpocketed. Send to a non-existing account to prevent auto receives from the wallet adjusting expected confirmation height
	nano::keypair key2;
	auto send2 = builder
				 .state ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (receive1->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				 .link (key2.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (receive1->hash ()))
				 .build ();

	{
		auto transaction = node->store.tx_begin_write ();
		ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send1));
		ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, receive1));
		ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send2));
	}

	// Call block confirm on the existing receive block on the genesis account which will confirm everything underneath on both accounts
	{
		node->scheduler.manual.push (receive1);
		std::shared_ptr<nano::election> election;
		ASSERT_TIMELY (10s, (election = node->active.election (receive1->qualified_root ())) != nullptr);
		node->active.force_confirm (*election);
	}

	ASSERT_TIMELY (30s, node->ledger.confirmed ().block_exists (*node->store.tx_begin_read (), receive1->hash ()));

	auto transaction (node->store.tx_begin_read ());
	auto info = node->ledger.any ().account_get (*transaction, nano::dev::genesis_key.pub);
	ASSERT_TRUE (info);
	nano::confirmation_height_info confirmation_height_info;
	ASSERT_FALSE (node->store.confirmation_height ().get (*transaction, nano::dev::genesis_key.pub, confirmation_height_info));
	ASSERT_EQ (num_blocks + 2, confirmation_height_info.height ());
	ASSERT_EQ (num_blocks + 3, info->block_count ()); // Includes the unpocketed send

	info = node->ledger.any ().account_get (*transaction, key1.pub);
	ASSERT_TRUE (info);
	ASSERT_FALSE (node->store.confirmation_height ().get (*transaction, key1.pub, confirmation_height_info));
	ASSERT_EQ (num_blocks + 1, confirmation_height_info.height ());
	ASSERT_EQ (num_blocks + 1, info->block_count ());

	size_t cemented_count = 0;
	for (auto i (node->ledger.store.confirmation_height ().begin (*transaction)), n (node->ledger.store.confirmation_height ().end ()); i != n; ++i)
	{
		cemented_count += i->second.height ();
	}

	ASSERT_EQ (cemented_count, node->ledger.cemented_count ());
	ASSERT_EQ (node->stats->count (nano::stat::type::confirmation_height, nano::stat::detail::blocks_confirmed, nano::stat::dir::in), num_blocks * 2 + 2);

	ASSERT_TIMELY_EQ (40s, (node->ledger.cemented_count () - 1), node->stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::all, nano::stat::dir::out));
	ASSERT_TIMELY_EQ (10s, node->active.election_winner_details_size (), 0);
}

TEST (confirmation_height, dynamic_algorithm)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	auto node = system.add_node (node_config);
	auto wallet_id = node->wallets.first_wallet_id ();
	nano::keypair key;
	(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	auto const num_blocks = nano::confirmation_height::unbounded_cutoff;
	auto latest_genesis = nano::dev::genesis;
	std::vector<std::shared_ptr<nano::state_block>> state_blocks;
	nano::block_builder builder;
	for (auto i = 0; i < num_blocks; ++i)
	{
		auto send = builder
					.state ()
					.account (nano::dev::genesis_key.pub)
					.previous (latest_genesis->hash ())
					.representative (nano::dev::genesis_key.pub)
					.balance (nano::dev::constants.genesis_amount - i - 1)
					.link (key.pub)
					.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					.work (*system.work.generate (latest_genesis->hash ()))
					.build ();
		latest_genesis = send;
		state_blocks.push_back (send);
	}
	{
		auto transaction = node->store.tx_begin_write ();
		for (auto const & block : state_blocks)
		{
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, block));
		}
	}

	node->confirming_set.add (state_blocks.front ()->hash ());
	ASSERT_TIMELY_EQ (20s, node->ledger.cemented_count (), 2);

	node->confirming_set.add (latest_genesis->hash ());

	ASSERT_TIMELY_EQ (20s, node->ledger.cemented_count (), num_blocks + 1);

	ASSERT_EQ (node->stats->count (nano::stat::type::confirmation_height, nano::stat::detail::blocks_confirmed, nano::stat::dir::in), num_blocks);
	ASSERT_TIMELY_EQ (10s, node->active.election_winner_details_size (), 0);
}

TEST (confirmation_height, many_accounts_send_receive_self)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.online_weight_minimum = 100;
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	node_config.active_elections.size = 400000;
	nano::node_flags node_flags;
	auto node = system.add_node (node_config);
	auto wallet_id = node->wallets.first_wallet_id ();
	(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);

#ifndef NDEBUG
	auto const num_accounts = 10000;
#else
	auto const num_accounts = 100000;
#endif

	auto latest_genesis = node->latest (nano::dev::genesis_key.pub);
	std::vector<nano::keypair> keys;
	nano::block_builder builder;
	std::vector<std::shared_ptr<nano::open_block>> open_blocks;
	{
		auto transaction = node->store.tx_begin_write ();
		for (auto i = 0; i < num_accounts; ++i)
		{
			nano::keypair key;
			keys.emplace_back (key);

			auto send = builder
						.send ()
						.previous (latest_genesis)
						.destination (key.pub)
						.balance (nano::dev::constants.genesis_amount - 1 - i)
						.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
						.work (*system.work.generate (latest_genesis))
						.build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send));
			auto open = builder
						.open ()
						.source (send->hash ())
						.representative (nano::dev::genesis_key.pub)
						.account (key.pub)
						.sign (key.prv, key.pub)
						.work (*system.work.generate (key.pub))
						.build ();
			ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, open));
			open_blocks.push_back (std::move (open));
			latest_genesis = send->hash ();
		}
	}

	// Confirm all of the accounts
	for (auto & open_block : open_blocks)
	{
		node->start_election (open_block);
		std::shared_ptr<nano::election> election;
		ASSERT_TIMELY (10s, (election = node->active.election (open_block->qualified_root ())) != nullptr);
		node->active.force_confirm (*election);
	}

	system.deadline_set (100s);
	auto num_blocks_to_confirm = num_accounts * 2;
	while (node->stats->count (nano::stat::type::confirmation_height, nano::stat::detail::blocks_confirmed, nano::stat::dir::in) != num_blocks_to_confirm)
	{
		ASSERT_NO_ERROR (system.poll ());
	}

	std::vector<std::shared_ptr<nano::send_block>> send_blocks;
	std::vector<std::shared_ptr<nano::receive_block>> receive_blocks;

	for (int i = 0; i < open_blocks.size (); ++i)
	{
		auto open_block = open_blocks[i];
		auto & keypair = keys[i];
		send_blocks.emplace_back (builder
								  .send ()
								  .previous (open_block->hash ())
								  .destination (keypair.pub)
								  .balance (1)
								  .sign (keypair.prv, keypair.pub)
								  .work (*system.work.generate (open_block->hash ()))
								  .build ());
		receive_blocks.emplace_back (builder
									 .receive ()
									 .previous (send_blocks.back ()->hash ())
									 .source (send_blocks.back ()->hash ())
									 .sign (keypair.prv, keypair.pub)
									 .work (*system.work.generate (send_blocks.back ()->hash ()))
									 .build ());
	}

	// Now send and receive to self
	for (int i = 0; i < open_blocks.size (); ++i)
	{
		node->process_active (send_blocks[i]);
		node->process_active (receive_blocks[i]);
	}

	system.deadline_set (300s);
	num_blocks_to_confirm = num_accounts * 4;
	while (node->stats->count (nano::stat::type::confirmation_height, nano::stat::detail::blocks_confirmed, nano::stat::dir::in) != num_blocks_to_confirm)
	{
		ASSERT_NO_ERROR (system.poll ());
	}

	system.deadline_set (200s);
	while ((node->ledger.cemented_count () - 1) != node->stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::all, nano::stat::dir::out))
	{
		ASSERT_NO_ERROR (system.poll ());
	}

	auto transaction = node->store.tx_begin_read ();
	size_t cemented_count = 0;
	for (auto i (node->ledger.store.confirmation_height ().begin (*transaction)), n (node->ledger.store.confirmation_height ().end ()); i != n; ++i)
	{
		cemented_count += i->second.height ();
	}

	ASSERT_EQ (num_blocks_to_confirm + 1, cemented_count);
	ASSERT_EQ (cemented_count, node->ledger.cemented_count ());

	system.deadline_set (60s);
	while ((node->ledger.cemented_count () - 1) != node->stats->count (nano::stat::type::confirmation_observer, nano::stat::detail::all, nano::stat::dir::out))
	{
		ASSERT_NO_ERROR (system.poll ());
	}

	system.deadline_set (60s);
	while (node->active.election_winner_details_size () > 0)
	{
		ASSERT_NO_ERROR (system.poll ());
	}
}

}

namespace
{
class data
{
public:
	std::atomic<bool> awaiting_cache{ false };
	std::atomic<bool> keep_requesting_metrics{ true };
	std::shared_ptr<nano::node> node;
	std::chrono::system_clock::time_point orig_time;
	std::atomic_flag orig_time_set = ATOMIC_FLAG_INIT;
};
class shared_data
{
public:
	nano::test::counted_completion write_completion{ 0 };
	std::atomic<bool> done{ false };
};

template <typename T>
void callback_process (shared_data & shared_data_a, data & data, T & all_node_data_a, std::chrono::system_clock::time_point last_updated)
{
	if (!data.orig_time_set.test_and_set ())
	{
		data.orig_time = last_updated;
	}

	if (data.awaiting_cache && data.orig_time != last_updated)
	{
		data.keep_requesting_metrics = false;
	}
	if (data.orig_time != last_updated)
	{
		data.awaiting_cache = true;
		data.orig_time = last_updated;
	}
	shared_data_a.write_completion.increment ();
};
}

TEST (telemetry, ongoing_requests)
{
	nano::test::system system;
	nano::node_flags node_flags;
	auto node_client = system.add_node (node_flags);
	auto node_server = system.add_node (node_flags);

	nano::test::wait_peer_connections (system);

	ASSERT_EQ (0, node_client->telemetry->size ());
	ASSERT_EQ (0, node_server->telemetry->size ());
	ASSERT_EQ (0, node_client->stats->count (nano::stat::type::bootstrap, nano::stat::detail::telemetry_ack, nano::stat::dir::in));
	ASSERT_EQ (0, node_client->stats->count (nano::stat::type::bootstrap, nano::stat::detail::telemetry_req, nano::stat::dir::out));

	ASSERT_TIMELY (20s, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in) == 1 && node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in) == 1);

	// Wait till the next ongoing will be called, and add a 1s buffer for the actual processing
	auto time = std::chrono::steady_clock::now ();
	ASSERT_TIMELY (10s, std::chrono::steady_clock::now () >= (time + nano::dev::network_params.network.telemetry_cache_cutoff + 1s));

	ASSERT_EQ (2, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in));
	ASSERT_EQ (2, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::in));
	ASSERT_EQ (2, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::out));
	ASSERT_EQ (2, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in));
	ASSERT_EQ (2, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::in));
	ASSERT_EQ (2, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::out));
}

TEST (telemetry, under_load)
{
	nano::test::system system;
	nano::node_config node_config = system.default_config ();
	node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
	nano::node_flags node_flags;
	auto node = system.add_node (node_config, node_flags);
	auto wallet_id = node->wallets.first_wallet_id ();
	node_config.peering_port = system.get_available_port ();
	auto node1 = system.add_node (node_config, node_flags);
	nano::keypair key;
	nano::keypair key1;
	(void)node->wallets.insert_adhoc (wallet_id, nano::dev::genesis_key.prv);
	(void)node->wallets.insert_adhoc (wallet_id, key.prv);
	auto latest_genesis = node->latest (nano::dev::genesis_key.pub);
	auto num_blocks = 150000;
	nano::block_builder builder;
	auto send = builder
				.state ()
				.account (nano::dev::genesis_key.pub)
				.previous (latest_genesis)
				.representative (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - num_blocks)
				.link (key.pub)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (latest_genesis))
				.build ();
	node->process_active (send);
	latest_genesis = send->hash ();
	auto open = builder
				.state ()
				.account (key.pub)
				.previous (0)
				.representative (key.pub)
				.balance (num_blocks)
				.link (send->hash ())
				.sign (key.prv, key.pub)
				.work (*system.work.generate (key.pub))
				.build ();
	node->process_active (open);
	auto latest_key = open->hash ();

	auto thread_func = [key1, &system, node, num_blocks] (nano::keypair const & keypair, nano::block_hash const & latest, nano::uint128_t const initial_amount) {
		auto latest_l = latest;
		nano::block_builder builder;
		for (int i = 0; i < num_blocks; ++i)
		{
			auto send = builder
						.state ()
						.account (keypair.pub)
						.previous (latest_l)
						.representative (keypair.pub)
						.balance (initial_amount - i - 1)
						.link (key1.pub)
						.sign (keypair.prv, keypair.pub)
						.work (*system.work.generate (latest_l))
						.build ();
			latest_l = send->hash ();
			node->process_active (send);
		}
	};

	std::thread thread1 (thread_func, nano::dev::genesis_key, latest_genesis, nano::dev::constants.genesis_amount - num_blocks);
	std::thread thread2 (thread_func, key, latest_key, num_blocks);

	ASSERT_TIMELY_EQ (200s, node1->ledger.block_count (), num_blocks * 2 + 3);

	thread1.join ();
	thread2.join ();

	for (auto const & node : system.nodes)
	{
		ASSERT_EQ (0, node->stats->count (nano::stat::type::telemetry, nano::stat::detail::failed_send_telemetry_req));
		ASSERT_EQ (0, node->stats->count (nano::stat::type::telemetry, nano::stat::detail::request_within_protection_cache_zone));
		ASSERT_EQ (0, node->stats->count (nano::stat::type::telemetry, nano::stat::detail::unsolicited_telemetry_ack));
		ASSERT_EQ (0, node->stats->count (nano::stat::type::telemetry, nano::stat::detail::no_response_received));
	}
}

/**
 * This test checks that the telemetry cached data is consistent and that it timeouts when it should.
 * It does the following:
 * It disables ongoing telemetry requests and creates 2 nodes, client and server.
 * The client node sends a manual telemetry req to the server node and waits for the telemetry reply.
 * The telemetry reply is saved in the callback and then it is also requested via nano::telemetry::get_metrics().
 * The 2 telemetry data obtained by the 2 different methods are checked that they are the same.
 * Then the test idles until the telemetry data timeouts from the cache.
 * Then the manual req and reply process is repeated and checked.
 */
TEST (telemetry, cache_read_and_timeout)
{
	nano::test::system system;
	nano::node_flags node_flags;
	node_flags.set_disable_ongoing_telemetry_requests (true);
	auto node_client = system.add_node (node_flags);
	auto node_server = system.add_node (node_flags);

	nano::test::wait_peer_connections (system);

	// Request telemetry metrics
	std::optional<nano::telemetry_data> telemetry_data;
	auto channel = node_client->network->find_node_id (node_server->get_node_id ());
	ASSERT_NE (channel, nullptr);

	node_client->telemetry->trigger ();
	ASSERT_TIMELY (5s, telemetry_data = node_client->telemetry->get_telemetry (channel->get_remote_endpoint ()));

	auto responses = node_client->telemetry->get_all_telemetries ();
	ASSERT_TRUE (!responses.empty ());
	ASSERT_EQ (telemetry_data, responses.begin ()->second);

	// Confirm only 1 request was made
	ASSERT_EQ (1, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in));
	ASSERT_EQ (0, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::in));
	ASSERT_EQ (1, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::out));
	ASSERT_EQ (0, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in));
	ASSERT_EQ (1, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::in));
	ASSERT_EQ (0, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::out));

	// wait until the telemetry data times out
	ASSERT_TIMELY (5s, node_client->telemetry->get_all_telemetries ().empty ());

	// the telemetry data cache should be empty now
	responses = node_client->telemetry->get_all_telemetries ();
	ASSERT_TRUE (responses.empty ());

	// Request telemetry metrics again
	node_client->telemetry->trigger ();
	ASSERT_TIMELY (5s, telemetry_data = node_client->telemetry->get_telemetry (channel->get_remote_endpoint ()));

	responses = node_client->telemetry->get_all_telemetries ();
	ASSERT_TRUE (!responses.empty ());
	ASSERT_EQ (telemetry_data, responses.begin ()->second);

	ASSERT_EQ (2, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in));
	ASSERT_EQ (0, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::in));
	ASSERT_EQ (2, node_client->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::out));
	ASSERT_EQ (0, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_ack, nano::stat::dir::in));
	ASSERT_EQ (2, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::in));
	ASSERT_EQ (0, node_server->stats->count (nano::stat::type::message, nano::stat::detail::telemetry_req, nano::stat::dir::out));
}

TEST (telemetry, many_nodes)
{
	nano::test::system system;
	nano::node_flags node_flags;
	node_flags.set_disable_request_loop (true);
	// The telemetry responses can timeout if using a large number of nodes under sanitizers, so lower the number.
	auto const num_nodes = nano::memory_intensive_instrumentation () ? 4 : 10;
	for (auto i = 0; i < num_nodes; ++i)
	{
		nano::node_config node_config = system.default_config ();
		// Make a metric completely different for each node so we can check afterwards that there are no duplicates
		node_config.bandwidth_limit = 100000 + i;

		auto node = std::make_shared<nano::node> (system.async_rt, nano::unique_path (), node_config, system.work, node_flags);
		node->start ();
		system.nodes.push_back (node);
	}

	// Merge peers after creating nodes as some backends (RocksDB) can take a while to initialize nodes (Windows/Debug for instance)
	// and timeouts can occur between nodes while starting up many nodes synchronously.
	for (auto const & node : system.nodes)
	{
		for (auto const & other_node : system.nodes)
		{
			if (node != other_node)
			{
				node->network->merge_peer (other_node->network->endpoint ());
			}
		}
	}

	nano::test::wait_peer_connections (system);

	// Give all nodes a non-default number of blocks
	nano::keypair key;
	nano::block_builder builder;
	auto send = builder
				.state ()
				.account (nano::dev::genesis_key.pub)
				.previous (nano::dev::genesis->hash ())
				.representative (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - nano::Mxrb_ratio)
				.link (key.pub)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (nano::dev::genesis->hash ()))
				.build ();
	for (auto node : system.nodes)
	{
		auto transaction (node->store.tx_begin_write ());
		ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send));
	}

	// This is the node which will request metrics from all other nodes
	auto node_client = system.nodes.front ();

	std::vector<nano::telemetry_data> telemetry_datas;
	auto peers = node_client->network->tcp_channels->list (num_nodes - 1);
	ASSERT_EQ (peers.size (), num_nodes - 1);
	for (auto const & peer : peers)
	{
		std::optional<nano::telemetry_data> telemetry_data;
		ASSERT_TIMELY (5s, telemetry_data = node_client->telemetry->get_telemetry (peer->get_remote_endpoint ()));
		telemetry_datas.push_back (*telemetry_data);
	}

	ASSERT_EQ (telemetry_datas.size (), num_nodes - 1);

	// Check the metrics
	for (auto & data : telemetry_datas)
	{
		ASSERT_EQ (data.get_unchecked_count (), 0);
		ASSERT_EQ (data.get_cemented_count (), 1);
		ASSERT_LE (data.get_peer_count (), 9U);
		ASSERT_EQ (data.get_account_count (), 1);
		ASSERT_EQ (data.get_block_count (), 2);
		ASSERT_EQ (data.get_protocol_version (), nano::dev::network_params.network.protocol_version);
		ASSERT_GE (data.get_bandwidth_cap (), 100000);
		ASSERT_LT (data.get_bandwidth_cap (), 100000 + system.nodes.size ());
		ASSERT_EQ (data.get_major_version (), nano::get_major_node_version ());
		ASSERT_EQ (data.get_minor_version (), nano::get_minor_node_version ());
		ASSERT_EQ (data.get_patch_version (), nano::get_patch_node_version ());
		ASSERT_EQ (data.get_pre_release_version (), nano::get_pre_release_node_version ());
		ASSERT_EQ (data.get_maker (), 0);
		ASSERT_LT (data.get_uptime (), 100);
		ASSERT_EQ (data.get_genesis_block (), nano::dev::genesis->hash ());
		ASSERT_LE (data.get_timestamp (), std::chrono::system_clock::now ());
		ASSERT_EQ (data.get_active_difficulty (), system.nodes.front ()->default_difficulty (nano::work_version::work_1));
	}

	// We gave some nodes different bandwidth caps, confirm they are not all the same
	auto bandwidth_cap = telemetry_datas.front ().get_bandwidth_cap ();
	telemetry_datas.erase (telemetry_datas.begin ());
	auto all_bandwidth_limits_same = std::all_of (telemetry_datas.begin (), telemetry_datas.end (), [bandwidth_cap] (auto & telemetry_data) {
		return telemetry_data.get_bandwidth_cap () == bandwidth_cap;
	});
	ASSERT_FALSE (all_bandwidth_limits_same);
}

namespace nano
{
TEST (node, send_single_many_peers)
{
	nano::test::system system (nano::memory_intensive_instrumentation () ? 4 : 10);
	nano::keypair key2;
	auto node0 = system.nodes[0];
	auto node1 = system.nodes[0];
	(void)node0->wallets.insert_adhoc (node0->wallets.first_wallet_id (), nano::dev::genesis_key.prv);
	(void)node1->wallets.insert_adhoc (node1->wallets.first_wallet_id (), key2.prv);
	ASSERT_NE (nullptr, node0->wallets.send_action (node0->wallets.first_wallet_id (), nano::dev::genesis_key.pub, key2.pub, system.nodes[0]->config->receive_minimum.number ()));
	ASSERT_EQ (std::numeric_limits<nano::uint128_t>::max () - system.nodes[0]->config->receive_minimum.number (), system.nodes[0]->balance (nano::dev::genesis_key.pub));
	ASSERT_TRUE (system.nodes[0]->balance (key2.pub).is_zero ());
	ASSERT_TIMELY (3.5min, std::all_of (system.nodes.begin (), system.nodes.end (), [&] (std::shared_ptr<nano::node> const & node_a) { return !node_a->balance (key2.pub).is_zero (); }));
	system.stop ();
	for (auto node : system.nodes)
	{
		ASSERT_TRUE (node->is_stopped ());
	}
}
}

TEST (node, wallet_create_block_confirm_conflicts)
{
	for (int i = 0; i < 5; ++i)
	{
		nano::test::system system;
		nano::block_builder builder;
		nano::node_config node_config (system.get_available_port ());
		node_config.frontiers_confirmation = nano::frontiers_confirmation_mode::disabled;
		auto node = system.add_node (node_config);
		auto const num_blocks = 10000;

		// First open the other account
		auto latest = nano::dev::genesis->hash ();
		nano::keypair key1;
		{
			auto transaction = node->store.tx_begin_write ();
			for (auto i = num_blocks - 1; i > 0; --i)
			{
				auto send = builder
							.send ()
							.previous (latest)
							.destination (key1.pub)
							.balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio + i + 1)
							.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
							.work (*system.work.generate (latest))
							.build ();
				ASSERT_EQ (nano::block_status::progress, node->ledger.process (*transaction, send));
				latest = send->hash ();
			}
		}

		// Keep creating wallets. This is to check that there is no issues present when confirming blocks at the same time.
		std::atomic<bool> done{ false };
		std::thread t ([node, &done] () {
			while (!done)
			{
				node->wallets.create (nano::random_wallet_id ());
			}
		});

		// Call block confirm on the top level send block which will confirm everything underneath on both accounts.
		{
			auto block = node->ledger.any ().block_get (*node->store.tx_begin_read (), latest);
			node->scheduler.manual.push (block);
			std::shared_ptr<nano::election> election;
			ASSERT_TIMELY (10s, (election = node->active.election (block->qualified_root ())) != nullptr);
			node->active.force_confirm (*election);
		}

		ASSERT_TIMELY (120s, node->ledger.confirmed ().block_exists (*node->store.tx_begin_read (), latest) && node->confirming_set.size () == 0);
		done = true;
		t.join ();
	}
}
