#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/lmdbconfig.hpp>
#include <nano/lib/logger_mt.hpp>
#include <nano/lib/stats.hpp>
#include <nano/lib/utility.hpp>
#include <nano/lib/work.hpp>
#include <nano/node/common.hpp>
#include <nano/node/lmdb/lmdb.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/secure/utility.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <boost/filesystem.hpp>

#include <fstream>
#include <unordered_set>

#include <stdlib.h>

using namespace std::chrono_literals;

TEST (block_store, construction)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
}

// already ported to rust
TEST (block_store, block_details)
{
	nano::block_details details_send (nano::epoch::epoch_0, true, false, false);
	ASSERT_TRUE (details_send.is_send ());
	ASSERT_FALSE (details_send.is_receive ());
	ASSERT_FALSE (details_send.is_epoch ());
	ASSERT_EQ (nano::epoch::epoch_0, details_send.epoch ());

	nano::block_details details_receive (nano::epoch::epoch_1, false, true, false);
	ASSERT_FALSE (details_receive.is_send ());
	ASSERT_TRUE (details_receive.is_receive ());
	ASSERT_FALSE (details_receive.is_epoch ());
	ASSERT_EQ (nano::epoch::epoch_1, details_receive.epoch ());

	nano::block_details details_epoch (nano::epoch::epoch_2, false, false, true);
	ASSERT_FALSE (details_epoch.is_send ());
	ASSERT_FALSE (details_epoch.is_receive ());
	ASSERT_TRUE (details_epoch.is_epoch ());
	ASSERT_EQ (nano::epoch::epoch_2, details_epoch.epoch ());

	nano::block_details details_none (nano::epoch::unspecified, false, false, false);
	ASSERT_FALSE (details_none.is_send ());
	ASSERT_FALSE (details_none.is_receive ());
	ASSERT_FALSE (details_none.is_epoch ());
	ASSERT_EQ (nano::epoch::unspecified, details_none.epoch ());
}

TEST (block_store, block_details_serialization)
{
	nano::block_details details1 (nano::epoch::epoch_2, false, true, false);
	std::vector<uint8_t> vector;
	{
		nano::vectorstream stream1 (vector);
		details1.serialize (stream1);
	}
	nano::bufferstream stream2 (vector.data (), vector.size ());
	nano::block_details details2;
	ASSERT_FALSE (details2.deserialize (stream2));
	ASSERT_EQ (details1, details2);
}

TEST (block_store, sideband_serialization)
{
	nano::block_details details;
	nano::block_sideband sideband1 (nano::account{ 1 }, nano::block_hash{ 4 }, nano::amount{ 2 }, 5, 3, details, nano::epoch::epoch_0);
	std::vector<uint8_t> vector;
	{
		nano::vectorstream stream1 (vector);
		sideband1.serialize (stream1, nano::block_type::receive);
	}
	nano::bufferstream stream2 (vector.data (), vector.size ());
	nano::block_sideband sideband2;
	ASSERT_FALSE (sideband2.deserialize (stream2, nano::block_type::receive));
	ASSERT_EQ (sideband1.account (), sideband2.account ());
	ASSERT_EQ (sideband1.balance (), sideband2.balance ());
	ASSERT_EQ (sideband1.height (), sideband2.height ());
	ASSERT_EQ (sideband1.successor (), sideband2.successor ());
	ASSERT_EQ (sideband1.timestamp (), sideband2.timestamp ());
}

TEST (block_store, add_item)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	auto block = builder
				 .open ()
				 .source (0)
				 .representative (1)
				 .account (0)
				 .sign (nano::keypair ().prv, 0)
				 .work (0)
				 .build ();
	block->sideband_set ({});
	auto hash1 (block->hash ());
	auto transaction (store->tx_begin_write ());
	auto latest1 (store->block ().get (*transaction, hash1));
	ASSERT_EQ (nullptr, latest1);
	ASSERT_FALSE (store->block ().exists (*transaction, hash1));
	store->block ().put (*transaction, hash1, *block);
	auto latest2 (store->block ().get (*transaction, hash1));
	ASSERT_NE (nullptr, latest2);
	ASSERT_EQ (*block, *latest2);
	ASSERT_TRUE (store->block ().exists (*transaction, hash1));
	ASSERT_FALSE (store->block ().exists (*transaction, hash1.number () - 1));
	store->block ().del (*transaction, hash1);
	auto latest3 (store->block ().get (*transaction, hash1));
	ASSERT_EQ (nullptr, latest3);
}

TEST (block_store, clear_successor)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	auto block1 = builder
				  .open ()
				  .source (0)
				  .representative (1)
				  .account (0)
				  .sign (nano::keypair ().prv, 0)
				  .work (0)
				  .build ();
	block1->sideband_set ({});
	auto transaction (store->tx_begin_write ());
	store->block ().put (*transaction, block1->hash (), *block1);
	auto block2 = builder
				  .open ()
				  .source (0)
				  .representative (2)
				  .account (0)
				  .sign (nano::keypair ().prv, 0)
				  .work (0)
				  .build ();
	block2->sideband_set ({});
	store->block ().put (*transaction, block2->hash (), *block2);
	auto block2_store (store->block ().get (*transaction, block1->hash ()));
	ASSERT_NE (nullptr, block2_store);
	ASSERT_EQ (0, block2_store->sideband ().successor ().number ());
	auto modified_sideband = block2_store->sideband ();
	modified_sideband.set_successor (block2->hash ());
	block1->sideband_set (modified_sideband);
	store->block ().put (*transaction, block1->hash (), *block1);
	{
		auto block1_store (store->block ().get (*transaction, block1->hash ()));
		ASSERT_NE (nullptr, block1_store);
		ASSERT_EQ (block2->hash (), block1_store->sideband ().successor ());
	}
	store->block ().successor_clear (*transaction, block1->hash ());
	{
		auto block1_store (store->block ().get (*transaction, block1->hash ()));
		ASSERT_NE (nullptr, block1_store);
		ASSERT_EQ (0, block1_store->sideband ().successor ().number ());
	}
}

TEST (block_store, add_nonempty_block)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::keypair key1;
	nano::block_builder builder;
	auto block = builder
				 .open ()
				 .source (0)
				 .representative (1)
				 .account (0)
				 .sign (nano::keypair ().prv, 0)
				 .work (0)
				 .build ();
	block->sideband_set ({});
	auto hash1 (block->hash ());
	block->signature_set (nano::sign_message (key1.prv, key1.pub, hash1));
	auto transaction (store->tx_begin_write ());
	auto latest1 (store->block ().get (*transaction, hash1));
	ASSERT_EQ (nullptr, latest1);
	store->block ().put (*transaction, hash1, *block);
	auto latest2 (store->block ().get (*transaction, hash1));
	ASSERT_NE (nullptr, latest2);
	ASSERT_EQ (*block, *latest2);
}

TEST (block_store, add_two_items)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::keypair key1;
	nano::block_builder builder;
	auto block = builder
				 .open ()
				 .source (0)
				 .representative (1)
				 .account (1)
				 .sign (nano::keypair ().prv, 0)
				 .work (0)
				 .build ();
	block->sideband_set ({});
	auto hash1 (block->hash ());
	block->signature_set (nano::sign_message (key1.prv, key1.pub, hash1));
	auto transaction (store->tx_begin_write ());
	auto latest1 (store->block ().get (*transaction, hash1));
	ASSERT_EQ (nullptr, latest1);
	auto block2 = builder
				  .open ()
				  .source (0)
				  .representative (1)
				  .account (3)
				  .sign (nano::keypair ().prv, 0)
				  .work (0)
				  .build ();
	block2->sideband_set ({});
	block2->account_set (3);
	auto hash2 (block2->hash ());
	block2->signature_set (nano::sign_message (key1.prv, key1.pub, hash2));
	auto latest2 (store->block ().get (*transaction, hash2));
	ASSERT_EQ (nullptr, latest2);
	store->block ().put (*transaction, hash1, *block);
	store->block ().put (*transaction, hash2, *block2);
	auto latest3 (store->block ().get (*transaction, hash1));
	ASSERT_NE (nullptr, latest3);
	ASSERT_EQ (*block, *latest3);
	auto latest4 (store->block ().get (*transaction, hash2));
	ASSERT_NE (nullptr, latest4);
	ASSERT_EQ (*block2, *latest4);
	ASSERT_FALSE (*latest3 == *latest4);
}

TEST (block_store, add_receive)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::keypair key1;
	nano::keypair key2;
	nano::block_builder builder;
	auto block1 = builder
				  .open ()
				  .source (0)
				  .representative (1)
				  .account (0)
				  .sign (nano::keypair ().prv, 0)
				  .work (0)
				  .build ();
	block1->sideband_set ({});
	auto transaction (store->tx_begin_write ());
	store->block ().put (*transaction, block1->hash (), *block1);
	auto block = builder
				 .receive ()
				 .previous (block1->hash ())
				 .source (1)
				 .sign (nano::keypair ().prv, 2)
				 .work (3)
				 .build ();
	block->sideband_set ({});
	nano::block_hash hash1 (block->hash ());
	auto latest1 (store->block ().get (*transaction, hash1));
	ASSERT_EQ (nullptr, latest1);
	store->block ().put (*transaction, hash1, *block);
	auto latest2 (store->block ().get (*transaction, hash1));
	ASSERT_NE (nullptr, latest2);
	ASSERT_EQ (*block, *latest2);
}

TEST (block_store, add_pending)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::keypair key1;
	nano::pending_key key2 (0, 0);
	nano::pending_info pending1;
	auto transaction (store->tx_begin_write ());
	ASSERT_TRUE (store->pending ().get (*transaction, key2, pending1));
	store->pending ().put (*transaction, key2, pending1);
	nano::pending_info pending2;
	ASSERT_FALSE (store->pending ().get (*transaction, key2, pending2));
	ASSERT_EQ (pending1, pending2);
	store->pending ().del (*transaction, key2);
	ASSERT_TRUE (store->pending ().get (*transaction, key2, pending2));
}

TEST (block_store, pending_iterator)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	auto transaction (store->tx_begin_write ());
	ASSERT_EQ (store->pending ().end (), store->pending ().begin (*transaction));
	store->pending ().put (*transaction, nano::pending_key (1, 2), { 2, 3, nano::epoch::epoch_1 });
	auto current (store->pending ().begin (*transaction));
	ASSERT_NE (store->pending ().end (), current);
	nano::pending_key key1 (current->first);
	ASSERT_EQ (nano::account (1), key1.account);
	ASSERT_EQ (nano::block_hash (2), key1.hash);
	nano::pending_info pending (current->second);
	ASSERT_EQ (nano::account (2), pending.source);
	ASSERT_EQ (nano::amount (3), pending.amount);
	ASSERT_EQ (nano::epoch::epoch_1, pending.epoch);
}

/**
 * Regression test for Issue 1164
 * This reconstructs the situation where a key is larger in pending than the account being iterated in pending_v1, leaving
 * iteration order up to the value, causing undefined behavior.
 * After the bugfix, the value is compared only if the keys are equal.
 */
TEST (block_store, pending_iterator_comparison)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::stat stats;
	auto transaction (store->tx_begin_write ());
	// Populate pending
	store->pending ().put (*transaction, nano::pending_key (nano::account (3), nano::block_hash (1)), nano::pending_info (nano::account (10), nano::amount (1), nano::epoch::epoch_0));
	store->pending ().put (*transaction, nano::pending_key (nano::account (3), nano::block_hash (4)), nano::pending_info (nano::account (10), nano::amount (0), nano::epoch::epoch_0));
	// Populate pending_v1
	store->pending ().put (*transaction, nano::pending_key (nano::account (2), nano::block_hash (2)), nano::pending_info (nano::account (10), nano::amount (2), nano::epoch::epoch_1));
	store->pending ().put (*transaction, nano::pending_key (nano::account (2), nano::block_hash (3)), nano::pending_info (nano::account (10), nano::amount (3), nano::epoch::epoch_1));

	// Iterate account 3 (pending)
	{
		size_t count = 0;
		nano::account begin (3);
		nano::account end (begin.number () + 1);
		for (auto i (store->pending ().begin (*transaction, nano::pending_key (begin, 0))), n (store->pending ().begin (*transaction, nano::pending_key (end, 0))); i != n; ++i, ++count)
		{
			nano::pending_key key (i->first);
			ASSERT_EQ (key.account, begin);
			ASSERT_LT (count, 3);
		}
		ASSERT_EQ (count, 2);
	}

	// Iterate account 2 (pending_v1)
	{
		size_t count = 0;
		nano::account begin (2);
		nano::account end (begin.number () + 1);
		for (auto i (store->pending ().begin (*transaction, nano::pending_key (begin, 0))), n (store->pending ().begin (*transaction, nano::pending_key (end, 0))); i != n; ++i, ++count)
		{
			nano::pending_key key (i->first);
			ASSERT_EQ (key.account, begin);
			ASSERT_LT (count, 3);
		}
		ASSERT_EQ (count, 2);
	}
}

TEST (block_store, genesis)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::ledger_cache ledger_cache;
	auto transaction (store->tx_begin_write ());
	store->initialize (*transaction, ledger_cache, nano::dev::constants);
	nano::account_info info;
	ASSERT_FALSE (store->account ().get (*transaction, nano::dev::genesis->account (), info));
	ASSERT_EQ (nano::dev::genesis->hash (), info.head ());
	auto block1 (store->block ().get (*transaction, info.head ()));
	ASSERT_NE (nullptr, block1);
	auto receive1 (dynamic_cast<nano::open_block *> (block1.get ()));
	ASSERT_NE (nullptr, receive1);
	ASSERT_LE (info.modified (), nano::seconds_since_epoch ());
	ASSERT_EQ (info.block_count (), 1);
	// Genesis block should be confirmed by default
	nano::confirmation_height_info confirmation_height_info;
	ASSERT_FALSE (store->confirmation_height ().get (*transaction, nano::dev::genesis->account (), confirmation_height_info));
	ASSERT_EQ (confirmation_height_info.height (), 1);
	ASSERT_EQ (confirmation_height_info.frontier (), nano::dev::genesis->hash ());
	auto dev_pub_text (nano::dev::genesis_key.pub.to_string ());
	auto dev_pub_account (nano::dev::genesis_key.pub.to_account ());
	auto dev_prv_text (nano::dev::genesis_key.prv.to_string ());
	ASSERT_EQ (nano::dev::genesis->account (), nano::dev::genesis_key.pub);
}

// This test checks for basic operations in the unchecked table such as putting a new block, retrieving it, and
// deleting it from the database
TEST (unchecked, simple)
{
	nano::test::system system{};
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	nano::unchecked_map unchecked{ *store, false };
	ASSERT_TRUE (!store->init_error ());
	nano::keypair key1;
	nano::block_builder builder;
	auto block = builder
				 .send ()
				 .previous (0)
				 .destination (1)
				 .balance (2)
				 .sign (key1.prv, key1.pub)
				 .work (5)
				 .build_shared ();
	// Asserts the block wasn't added yet to the unchecked table
	auto block_listing1 = unchecked.get (*store->tx_begin_read (), block->previous ());
	ASSERT_TRUE (block_listing1.empty ());
	// Enqueues a block to be saved on the unchecked table
	unchecked.put (block->previous (), nano::unchecked_info (block));
	// Waits for the block to get written in the database
	auto check_block_is_listed = [&] (nano::transaction const & transaction_a, nano::block_hash const & block_hash_a) {
		return unchecked.get (transaction_a, block_hash_a).size () > 0;
	};
	ASSERT_TIMELY (5s, check_block_is_listed (*store->tx_begin_read (), block->previous ()));
	auto transaction = store->tx_begin_write ();
	// Retrieves the block from the database
	auto block_listing2 = unchecked.get (*transaction, block->previous ());
	ASSERT_FALSE (block_listing2.empty ());
	// Asserts the added block is equal to the retrieved one
	ASSERT_EQ (*block, *(block_listing2[0].get_block ()));
	// Deletes the block from the database
	unchecked.del (*transaction, nano::unchecked_key (block->previous (), block->hash ()));
	// Asserts the block is deleted
	auto block_listing3 = unchecked.get (*transaction, block->previous ());
	ASSERT_TRUE (block_listing3.empty ());
}

// This test ensures the unchecked table is able to receive more than one block
TEST (unchecked, multiple)
{
	nano::test::system system{};
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	nano::unchecked_map unchecked{ *store, false };
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	nano::keypair key1;
	auto block = builder
				 .send ()
				 .previous (4)
				 .destination (1)
				 .balance (2)
				 .sign (key1.prv, key1.pub)
				 .work (5)
				 .build_shared ();
	// Asserts the block wasn't added yet to the unchecked table
	auto block_listing1 = unchecked.get (*store->tx_begin_read (), block->previous ());
	ASSERT_TRUE (block_listing1.empty ());
	// Enqueues the first block
	unchecked.put (block->previous (), nano::unchecked_info (block));
	// Enqueues a second block
	unchecked.put (block->source (), nano::unchecked_info (block));
	auto check_block_is_listed = [&] (nano::transaction const & transaction_a, nano::block_hash const & block_hash_a) {
		return unchecked.get (transaction_a, block_hash_a).size () > 0;
	};
	// Waits for and asserts the first block gets saved in the database
	ASSERT_TIMELY (5s, check_block_is_listed (*store->tx_begin_read (), block->previous ()));
	// Waits for and asserts the second block gets saved in the database
	ASSERT_TIMELY (5s, check_block_is_listed (*store->tx_begin_read (), block->source ()));
}

// This test ensures that a block can't occur twice in the unchecked table.
TEST (unchecked, double_put)
{
	nano::test::system system{};
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	nano::unchecked_map unchecked{ *store, false };
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	nano::keypair key1;
	auto block = builder
				 .send ()
				 .previous (4)
				 .destination (1)
				 .balance (2)
				 .sign (key1.prv, key1.pub)
				 .work (5)
				 .build_shared ();
	// Asserts the block wasn't added yet to the unchecked table
	auto block_listing1 = unchecked.get (*store->tx_begin_read (), block->previous ());
	ASSERT_TRUE (block_listing1.empty ());
	// Enqueues the block to be saved in the unchecked table
	unchecked.put (block->previous (), nano::unchecked_info (block));
	// Enqueues the block again in an attempt to have it there twice
	unchecked.put (block->previous (), nano::unchecked_info (block));
	auto check_block_is_listed = [&] (nano::transaction const & transaction_a, nano::block_hash const & block_hash_a) {
		return unchecked.get (transaction_a, block_hash_a).size () > 0;
	};
	// Waits for and asserts the block was added at least once
	ASSERT_TIMELY (5s, check_block_is_listed (*store->tx_begin_read (), block->previous ()));
	// Asserts the block was added at most once -- this is objective of this test.
	auto block_listing2 = unchecked.get (*store->tx_begin_read (), block->previous ());
	ASSERT_EQ (block_listing2.size (), 1);
}

// Tests that recurrent get calls return the correct values
TEST (unchecked, multiple_get)
{
	nano::test::system system{};
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	nano::unchecked_map unchecked{ *store, false };
	ASSERT_TRUE (!store->init_error ());
	// Instantiates three blocks
	nano::keypair key1;
	nano::keypair key2;
	nano::keypair key3;
	nano::block_builder builder;
	auto block1 = builder
				  .send ()
				  .previous (4)
				  .destination (1)
				  .balance (2)
				  .sign (key1.prv, key1.pub)
				  .work (5)
				  .build_shared ();
	auto block2 = builder
				  .send ()
				  .previous (3)
				  .destination (1)
				  .balance (2)
				  .sign (key2.prv, key2.pub)
				  .work (5)
				  .build_shared ();
	auto block3 = builder
				  .send ()
				  .previous (5)
				  .destination (1)
				  .balance (2)
				  .sign (key3.prv, key3.pub)
				  .work (5)
				  .build_shared ();
	// Add the blocks' info to the unchecked table
	unchecked.put (block1->previous (), nano::unchecked_info (block1)); // unchecked1
	unchecked.put (block1->hash (), nano::unchecked_info (block1)); // unchecked2
	unchecked.put (block2->previous (), nano::unchecked_info (block2)); // unchecked3
	unchecked.put (block1->previous (), nano::unchecked_info (block2)); // unchecked1
	unchecked.put (block1->hash (), nano::unchecked_info (block2)); // unchecked2
	unchecked.put (block3->previous (), nano::unchecked_info (block3));
	unchecked.put (block3->hash (), nano::unchecked_info (block3)); // unchecked4
	unchecked.put (block1->previous (), nano::unchecked_info (block3)); // unchecked1

	// count the number of blocks in the unchecked table by counting them one by one
	// we cannot trust the count() method if the backend is rocksdb
	auto count_unchecked_blocks_one_by_one = [&store, &unchecked] () {
		size_t count = 0;
		auto transaction = store->tx_begin_read ();
		unchecked.for_each (*transaction, [&count] (nano::unchecked_key const & key, nano::unchecked_info const & info) {
			++count;
		});
		return count;
	};

	// Waits for the blocks to get saved in the database
	ASSERT_TIMELY (5s, 8 == count_unchecked_blocks_one_by_one ());

	std::vector<nano::block_hash> unchecked1;
	// Asserts the entries will be found for the provided key
	auto transaction = store->tx_begin_read ();
	auto unchecked1_blocks = unchecked.get (*transaction, block1->previous ());
	ASSERT_EQ (unchecked1_blocks.size (), 3);
	for (auto & i : unchecked1_blocks)
	{
		unchecked1.push_back (i.get_block ()->hash ());
	}
	// Asserts the payloads where correclty saved
	ASSERT_TRUE (std::find (unchecked1.begin (), unchecked1.end (), block1->hash ()) != unchecked1.end ());
	ASSERT_TRUE (std::find (unchecked1.begin (), unchecked1.end (), block2->hash ()) != unchecked1.end ());
	ASSERT_TRUE (std::find (unchecked1.begin (), unchecked1.end (), block3->hash ()) != unchecked1.end ());
	std::vector<nano::block_hash> unchecked2;
	// Asserts the entries will be found for the provided key
	auto unchecked2_blocks = unchecked.get (*transaction, block1->hash ());
	ASSERT_EQ (unchecked2_blocks.size (), 2);
	for (auto & i : unchecked2_blocks)
	{
		unchecked2.push_back (i.get_block ()->hash ());
	}
	// Asserts the payloads where correctly saved
	ASSERT_TRUE (std::find (unchecked2.begin (), unchecked2.end (), block1->hash ()) != unchecked2.end ());
	ASSERT_TRUE (std::find (unchecked2.begin (), unchecked2.end (), block2->hash ()) != unchecked2.end ());
	// Asserts the entry is found by the key and the payload is saved
	auto unchecked3 = unchecked.get (*transaction, block2->previous ());
	ASSERT_EQ (unchecked3.size (), 1);
	ASSERT_EQ (unchecked3[0].get_block ()->hash (), block2->hash ());
	// Asserts the entry is found by the key and the payload is saved
	auto unchecked4 = unchecked.get (*transaction, block3->hash ());
	ASSERT_EQ (unchecked4.size (), 1);
	ASSERT_EQ (unchecked4[0].get_block ()->hash (), block3->hash ());
	// Asserts no entry is found for a block that wasn't added
	auto unchecked5 = unchecked.get (*transaction, block2->hash ());
	ASSERT_EQ (unchecked5.size (), 0);
}

TEST (block_store, empty_accounts)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	auto transaction (store->tx_begin_read ());
	auto begin (store->account ().begin (*transaction));
	auto end (store->account ().end ());
	ASSERT_EQ (end, begin);
}

TEST (block_store, one_block)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	auto block1 = builder
				  .open ()
				  .source (0)
				  .representative (1)
				  .account (0)
				  .sign (nano::keypair ().prv, 0)
				  .work (0)
				  .build ();
	block1->sideband_set ({});
	auto transaction (store->tx_begin_write ());
	store->block ().put (*transaction, block1->hash (), *block1);
	ASSERT_TRUE (store->block ().exists (*transaction, block1->hash ()));
}

TEST (block_store, empty_bootstrap)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	nano::unchecked_map unchecked{ *store, false };
	ASSERT_TRUE (!store->init_error ());
	auto transaction (store->tx_begin_read ());
	size_t count = 0;
	unchecked.for_each (*transaction, [&count] (nano::unchecked_key const & key, nano::unchecked_info const & info) {
		++count;
	});
	ASSERT_EQ (count, 0);
}

TEST (block_store, unchecked_begin_search)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::keypair key0;
	nano::block_builder builder;
	auto block1 = builder
				  .send ()
				  .previous (0)
				  .destination (1)
				  .balance (2)
				  .sign (key0.prv, key0.pub)
				  .work (3)
				  .build ();
	auto block2 = builder
				  .send ()
				  .previous (5)
				  .destination (6)
				  .balance (7)
				  .sign (key0.prv, key0.pub)
				  .work (8)
				  .build ();
}

TEST (block_store, frontier_retrieval)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::account account1{};
	nano::account_info info1 (0, 0, 0, 0, 0, 0, nano::epoch::epoch_0);
	auto transaction (store->tx_begin_write ());
	store->confirmation_height ().put (*transaction, account1, { 0, nano::block_hash (0) });
	store->account ().put (*transaction, account1, info1);
	nano::account_info info2;
	store->account ().get (*transaction, account1, info2);
	ASSERT_EQ (info1, info2);
}

TEST (block_store, one_account)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::account account{};
	nano::block_hash hash (0);
	auto transaction (store->tx_begin_write ());
	store->confirmation_height ().put (*transaction, account, { 20, nano::block_hash (15) });
	store->account ().put (*transaction, account, { hash, account, hash, 42, 100, 200, nano::epoch::epoch_0 });
	auto begin (store->account ().begin (*transaction));
	auto end (store->account ().end ());
	ASSERT_NE (end, begin);
	ASSERT_EQ (account, nano::account (begin->first));
	nano::account_info info (begin->second);
	ASSERT_EQ (hash, info.head ());
	ASSERT_EQ (42, info.balance ().number ());
	ASSERT_EQ (100, info.modified ());
	ASSERT_EQ (200, info.block_count ());
	nano::confirmation_height_info confirmation_height_info;
	ASSERT_FALSE (store->confirmation_height ().get (*transaction, account, confirmation_height_info));
	ASSERT_EQ (20, confirmation_height_info.height ());
	ASSERT_EQ (nano::block_hash (15), confirmation_height_info.frontier ());
	++begin;
	ASSERT_EQ (end, begin);
}

TEST (block_store, two_block)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	auto block1 = builder
				  .open ()
				  .source (0)
				  .representative (1)
				  .account (1)
				  .sign (nano::keypair ().prv, 0)
				  .work (0)
				  .build ();
	block1->sideband_set ({});
	block1->account_set (1);
	std::vector<nano::block_hash> hashes;
	std::vector<nano::open_block> blocks;
	hashes.push_back (block1->hash ());
	blocks.push_back (*block1);
	auto transaction (store->tx_begin_write ());
	store->block ().put (*transaction, hashes[0], *block1);
	auto block2 = builder
				  .open ()
				  .source (0)
				  .representative (1)
				  .account (2)
				  .sign (nano::keypair ().prv, 0)
				  .work (0)
				  .build ();
	block2->sideband_set ({});
	hashes.push_back (block2->hash ());
	blocks.push_back (*block2);
	store->block ().put (*transaction, hashes[1], *block2);
	ASSERT_TRUE (store->block ().exists (*transaction, block1->hash ()));
	ASSERT_TRUE (store->block ().exists (*transaction, block2->hash ()));
}

TEST (block_store, two_account)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::account account1 (1);
	nano::block_hash hash1 (2);
	nano::account account2 (3);
	nano::block_hash hash2 (4);
	auto transaction (store->tx_begin_write ());
	store->confirmation_height ().put (*transaction, account1, { 20, nano::block_hash (10) });
	store->account ().put (*transaction, account1, { hash1, account1, hash1, 42, 100, 300, nano::epoch::epoch_0 });
	store->confirmation_height ().put (*transaction, account2, { 30, nano::block_hash (20) });
	store->account ().put (*transaction, account2, { hash2, account2, hash2, 84, 200, 400, nano::epoch::epoch_0 });
	auto begin (store->account ().begin (*transaction));
	auto end (store->account ().end ());
	ASSERT_NE (end, begin);
	ASSERT_EQ (account1, nano::account (begin->first));
	nano::account_info info1 (begin->second);
	ASSERT_EQ (hash1, info1.head ());
	ASSERT_EQ (42, info1.balance ().number ());
	ASSERT_EQ (100, info1.modified ());
	ASSERT_EQ (300, info1.block_count ());
	nano::confirmation_height_info confirmation_height_info;
	ASSERT_FALSE (store->confirmation_height ().get (*transaction, account1, confirmation_height_info));
	ASSERT_EQ (20, confirmation_height_info.height ());
	ASSERT_EQ (nano::block_hash (10), confirmation_height_info.frontier ());
	++begin;
	ASSERT_NE (end, begin);
	ASSERT_EQ (account2, nano::account (begin->first));
	nano::account_info info2 (begin->second);
	ASSERT_EQ (hash2, info2.head ());
	ASSERT_EQ (84, info2.balance ().number ());
	ASSERT_EQ (200, info2.modified ());
	ASSERT_EQ (400, info2.block_count ());
	ASSERT_FALSE (store->confirmation_height ().get (*transaction, account2, confirmation_height_info));
	ASSERT_EQ (30, confirmation_height_info.height ());
	ASSERT_EQ (nano::block_hash (20), confirmation_height_info.frontier ());
	++begin;
	ASSERT_EQ (end, begin);
}

TEST (block_store, latest_find)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::account account1 (1);
	nano::block_hash hash1 (2);
	nano::account account2 (3);
	nano::block_hash hash2 (4);
	auto transaction (store->tx_begin_write ());
	store->confirmation_height ().put (*transaction, account1, { 0, nano::block_hash (0) });
	store->account ().put (*transaction, account1, { hash1, account1, hash1, 100, 0, 300, nano::epoch::epoch_0 });
	store->confirmation_height ().put (*transaction, account2, { 0, nano::block_hash (0) });
	store->account ().put (*transaction, account2, { hash2, account2, hash2, 200, 0, 400, nano::epoch::epoch_0 });
	auto first (store->account ().begin (*transaction));
	auto second (store->account ().begin (*transaction));
	++second;
	auto find1 (store->account ().begin (*transaction, 1));
	ASSERT_EQ (first, find1);
	auto find2 (store->account ().begin (*transaction, 3));
	ASSERT_EQ (second, find2);
	auto find3 (store->account ().begin (*transaction, 2));
	ASSERT_EQ (second, find3);
}

namespace nano
{
namespace lmdb
{
	TEST (mdb_block_store, supported_version_upgrades)
	{
		// Check that upgrading from an unsupported version is not supported
		auto path (nano::unique_path ());
		auto logger{ std::make_shared<nano::logger_mt> () };
		{
			nano::lmdb::store store (logger, path, nano::dev::constants);
			nano::stat stats;
			nano::ledger ledger (store, stats, nano::dev::constants);
			auto transaction (store.tx_begin_write ());
			store.initialize (*transaction, ledger.cache, nano::dev::constants);
			// Lower the database to the max version unsupported for upgrades
			store.version ().put (*transaction, store.version_minimum - 1);
		}

		// Upgrade should fail
		{
			nano::lmdb::store store (logger, path, nano::dev::constants);
			ASSERT_TRUE (store.init_error ());
		}
	}
}
}

TEST (mdb_block_store, bad_path)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	nano::lmdb::store store (logger, boost::filesystem::path ("///"), nano::dev::constants);
	ASSERT_TRUE (store.init_error ());
}

TEST (block_store, DISABLED_already_open) // File can be shared
{
	auto path (nano::unique_path ());
	boost::filesystem::create_directories (path.parent_path ());
	nano::set_secure_perm_directory (path.parent_path ());
	std::ofstream file;
	file.open (path.string ().c_str ());
	ASSERT_TRUE (file.is_open ());
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, path, nano::dev::constants);
	ASSERT_TRUE (store->init_error ());
}

TEST (block_store, roots)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	nano::keypair key1;
	nano::keypair key2;
	nano::keypair key3;
	nano::keypair key4;
	auto send_block = builder
					  .send ()
					  .previous (0)
					  .destination (1)
					  .balance (2)
					  .sign (key1.prv, key1.pub)
					  .work (5)
					  .build ();
	ASSERT_EQ (send_block->previous (), send_block->root ());
	auto change_block = builder
						.change ()
						.previous (0)
						.representative (1)
						.sign (key2.prv, key2.pub)
						.work (4)
						.build ();
	ASSERT_EQ (change_block->previous (), change_block->root ());
	auto receive_block = builder
						 .receive ()
						 .previous (0)
						 .source (1)
						 .sign (key3.prv, key3.pub)
						 .work (4)
						 .build ();
	ASSERT_EQ (receive_block->previous (), receive_block->root ());
	auto open_block = builder
					  .open ()
					  .source (0)
					  .representative (1)
					  .account (2)
					  .sign (key4.prv, key4.pub)
					  .work (5)
					  .build ();
	ASSERT_EQ (open_block->account (), open_block->root ());
}

TEST (block_store, pending_exists)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::pending_key two (2, 0);
	nano::pending_info pending;
	auto transaction (store->tx_begin_write ());
	store->pending ().put (*transaction, two, pending);
	nano::pending_key one (1, 0);
	ASSERT_FALSE (store->pending ().exists (*transaction, one));
}

TEST (block_store, latest_exists)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::account two (2);
	nano::account_info info;
	auto transaction (store->tx_begin_write ());
	store->confirmation_height ().put (*transaction, two, { 0, nano::block_hash (0) });
	store->account ().put (*transaction, two, info);
	nano::account one (1);
	ASSERT_FALSE (store->account ().exists (*transaction, one));
}

TEST (block_store, large_iteration)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	std::unordered_set<nano::account> accounts1;
	for (auto i (0); i < 1000; ++i)
	{
		auto transaction (store->tx_begin_write ());
		nano::account account;
		nano::random_pool::generate_block (account.bytes.data (), account.bytes.size ());
		accounts1.insert (account);
		store->confirmation_height ().put (*transaction, account, { 0, nano::block_hash (0) });
		store->account ().put (*transaction, account, nano::account_info ());
	}
	std::unordered_set<nano::account> accounts2;
	nano::account previous{};
	auto transaction (store->tx_begin_read ());
	for (auto i (store->account ().begin (*transaction, 0)), n (store->account ().end ()); i != n; ++i)
	{
		nano::account current (i->first);
		ASSERT_GT (current.number (), previous.number ());
		accounts2.insert (current);
		previous = current;
	}
	ASSERT_EQ (accounts1, accounts2);
	// Reverse iteration
	std::unordered_set<nano::account> accounts3;
	previous = std::numeric_limits<nano::uint256_t>::max ();
	for (auto i (store->account ().rbegin (*transaction)), n (store->account ().end ()); i != n; --i)
	{
		nano::account current (i->first);
		ASSERT_LT (current.number (), previous.number ());
		accounts3.insert (current);
		previous = current;
	}
	ASSERT_EQ (accounts1, accounts3);
}

TEST (block_store, frontier)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	auto transaction (store->tx_begin_write ());
	nano::block_hash hash (100);
	nano::account account (200);
	ASSERT_TRUE (store->frontier ().get (*transaction, hash).is_zero ());
	store->frontier ().put (*transaction, hash, account);
	ASSERT_EQ (account, store->frontier ().get (*transaction, hash));
	store->frontier ().del (*transaction, hash);
	ASSERT_TRUE (store->frontier ().get (*transaction, hash).is_zero ());
}

TEST (block_store, block_replace)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	auto send1 = builder
				 .send ()
				 .previous (0)
				 .destination (0)
				 .balance (0)
				 .sign (nano::keypair ().prv, 0)
				 .work (1)
				 .build ();
	send1->sideband_set ({});
	auto send2 = builder
				 .send ()
				 .previous (0)
				 .destination (0)
				 .balance (0)
				 .sign (nano::keypair ().prv, 0)
				 .work (2)
				 .build ();
	send2->sideband_set ({});
	auto transaction (store->tx_begin_write ());
	store->block ().put (*transaction, 0, *send1);
	store->block ().put (*transaction, 0, *send2);
	auto block3 (store->block ().get (*transaction, 0));
	ASSERT_NE (nullptr, block3);
	ASSERT_EQ (2, block3->block_work ());
}

TEST (block_store, block_count)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	{
		auto transaction (store->tx_begin_write ());
		ASSERT_EQ (0, store->block ().count (*transaction));
		nano::block_builder builder;
		auto block = builder
					 .open ()
					 .source (0)
					 .representative (1)
					 .account (0)
					 .sign (nano::keypair ().prv, 0)
					 .work (0)
					 .build ();
		block->sideband_set ({});
		auto hash1 (block->hash ());
		store->block ().put (*transaction, hash1, *block);
	}
	auto transaction (store->tx_begin_read ());
	ASSERT_EQ (1, store->block ().count (*transaction));
}

TEST (block_store, account_count)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	{
		auto transaction (store->tx_begin_write ());
		ASSERT_EQ (0, store->account ().count (*transaction));
		nano::account account (200);
		store->confirmation_height ().put (*transaction, account, { 0, nano::block_hash (0) });
		store->account ().put (*transaction, account, nano::account_info ());
	}
	auto transaction (store->tx_begin_read ());
	ASSERT_EQ (1, store->account ().count (*transaction));
}

TEST (block_store, cemented_count_cache)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	auto transaction (store->tx_begin_write ());
	nano::ledger_cache ledger_cache;
	store->initialize (*transaction, ledger_cache, nano::dev::constants);
	ASSERT_EQ (1, ledger_cache.cemented_count);
}

TEST (block_store, block_random)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	{
		nano::ledger_cache ledger_cache;
		auto transaction (store->tx_begin_write ());
		store->initialize (*transaction, ledger_cache, nano::dev::constants);
	}
	auto transaction (store->tx_begin_read ());
	auto block (store->block ().random (*transaction));
	ASSERT_NE (nullptr, block);
	ASSERT_EQ (*block, *nano::dev::genesis);
}

TEST (block_store, pruned_random)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());
	nano::block_builder builder;
	auto block = builder
				 .open ()
				 .source (0)
				 .representative (1)
				 .account (0)
				 .sign (nano::keypair ().prv, 0)
				 .work (0)
				 .build ();
	block->sideband_set ({});
	auto hash1 (block->hash ());
	{
		nano::ledger_cache ledger_cache;
		auto transaction (store->tx_begin_write ());
		store->initialize (*transaction, ledger_cache, nano::dev::constants);
		store->pruned ().put (*transaction, hash1);
	}
	auto transaction (store->tx_begin_read ());
	auto random_hash (store->pruned ().random (*transaction));
	ASSERT_EQ (hash1, random_hash);
}

TEST (block_store, state_block)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_FALSE (store->init_error ());
	nano::keypair key1;
	nano::block_builder builder;
	auto block1 = builder
				  .state ()
				  .account (1)
				  .previous (nano::dev::genesis->hash ())
				  .representative (3)
				  .balance (4)
				  .link (6)
				  .sign (key1.prv, key1.pub)
				  .work (7)
				  .build ();

	block1->sideband_set ({});
	{
		nano::ledger_cache ledger_cache;
		auto transaction (store->tx_begin_write ());
		store->initialize (*transaction, ledger_cache, nano::dev::constants);
		ASSERT_EQ (nano::block_type::state, block1->type ());
		store->block ().put (*transaction, block1->hash (), *block1);
		ASSERT_TRUE (store->block ().exists (*transaction, block1->hash ()));
		auto block2 (store->block ().get (*transaction, block1->hash ()));
		ASSERT_NE (nullptr, block2);
		ASSERT_EQ (*block1, *block2);
	}
	{
		auto transaction (store->tx_begin_write ());
		auto count (store->block ().count (*transaction));
		ASSERT_EQ (2, count);
		store->block ().del (*transaction, block1->hash ());
		ASSERT_FALSE (store->block ().exists (*transaction, block1->hash ()));
	}
	auto transaction (store->tx_begin_read ());
	auto count2 (store->block ().count (*transaction));
	ASSERT_EQ (1, count2);
}

TEST (mdb_block_store, sideband_height)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	nano::keypair key1;
	nano::keypair key2;
	nano::keypair key3;
	nano::lmdb::store store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_FALSE (store.init_error ());
	nano::stat stat;
	nano::ledger ledger (store, stat, nano::dev::constants);
	nano::block_builder builder;
	auto transaction (store.tx_begin_write ());
	store.initialize (*transaction, ledger.cache, nano::dev::constants);
	nano::work_pool pool{ nano::dev::network_params.network, std::numeric_limits<unsigned>::max () };
	auto send = builder
				.send ()
				.previous (nano::dev::genesis->hash ())
				.destination (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*pool.generate (nano::dev::genesis->hash ()))
				.build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *send).code);
	auto receive = builder
				   .receive ()
				   .previous (send->hash ())
				   .source (send->hash ())
				   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				   .work (*pool.generate (send->hash ()))
				   .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *receive).code);
	auto change = builder
				  .change ()
				  .previous (receive->hash ())
				  .representative (0)
				  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				  .work (*pool.generate (receive->hash ()))
				  .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *change).code);
	auto state_send1 = builder
					   .state ()
					   .account (nano::dev::genesis_key.pub)
					   .previous (change->hash ())
					   .representative (0)
					   .balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
					   .link (key1.pub)
					   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					   .work (*pool.generate (change->hash ()))
					   .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *state_send1).code);
	auto state_send2 = builder
					   .state ()
					   .account (nano::dev::genesis_key.pub)
					   .previous (state_send1->hash ())
					   .representative (0)
					   .balance (nano::dev::constants.genesis_amount - 2 * nano::Gxrb_ratio)
					   .link (key2.pub)
					   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					   .work (*pool.generate (state_send1->hash ()))
					   .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *state_send2).code);
	auto state_send3 = builder
					   .state ()
					   .account (nano::dev::genesis_key.pub)
					   .previous (state_send2->hash ())
					   .representative (0)
					   .balance (nano::dev::constants.genesis_amount - 3 * nano::Gxrb_ratio)
					   .link (key3.pub)
					   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					   .work (*pool.generate (state_send2->hash ()))
					   .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *state_send3).code);
	auto state_open = builder
					  .state ()
					  .account (key1.pub)
					  .previous (0)
					  .representative (0)
					  .balance (nano::Gxrb_ratio)
					  .link (state_send1->hash ())
					  .sign (key1.prv, key1.pub)
					  .work (*pool.generate (key1.pub))
					  .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *state_open).code);
	auto epoch = builder
				 .state ()
				 .account (key1.pub)
				 .previous (state_open->hash ())
				 .representative (0)
				 .balance (nano::Gxrb_ratio)
				 .link (ledger.epoch_link (nano::epoch::epoch_1))
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*pool.generate (state_open->hash ()))
				 .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *epoch).code);
	ASSERT_EQ (nano::epoch::epoch_1, store.block ().version (*transaction, epoch->hash ()));
	auto epoch_open = builder
					  .state ()
					  .account (key2.pub)
					  .previous (0)
					  .representative (0)
					  .balance (0)
					  .link (ledger.epoch_link (nano::epoch::epoch_1))
					  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					  .work (*pool.generate (key2.pub))
					  .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *epoch_open).code);
	ASSERT_EQ (nano::epoch::epoch_1, store.block ().version (*transaction, epoch_open->hash ()));
	auto state_receive = builder
						 .state ()
						 .account (key2.pub)
						 .previous (epoch_open->hash ())
						 .representative (0)
						 .balance (nano::Gxrb_ratio)
						 .link (state_send2->hash ())
						 .sign (key2.prv, key2.pub)
						 .work (*pool.generate (epoch_open->hash ()))
						 .build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *state_receive).code);
	auto open = builder
				.open ()
				.source (state_send3->hash ())
				.representative (nano::dev::genesis_key.pub)
				.account (key3.pub)
				.sign (key3.prv, key3.pub)
				.work (*pool.generate (key3.pub))
				.build ();
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *open).code);
	auto block1 (store.block ().get (*transaction, nano::dev::genesis->hash ()));
	ASSERT_EQ (block1->sideband ().height (), 1);
	auto block2 (store.block ().get (*transaction, send->hash ()));
	ASSERT_EQ (block2->sideband ().height (), 2);
	auto block3 (store.block ().get (*transaction, receive->hash ()));
	ASSERT_EQ (block3->sideband ().height (), 3);
	auto block4 (store.block ().get (*transaction, change->hash ()));
	ASSERT_EQ (block4->sideband ().height (), 4);
	auto block5 (store.block ().get (*transaction, state_send1->hash ()));
	ASSERT_EQ (block5->sideband ().height (), 5);
	auto block6 (store.block ().get (*transaction, state_send2->hash ()));
	ASSERT_EQ (block6->sideband ().height (), 6);
	auto block7 (store.block ().get (*transaction, state_send3->hash ()));
	ASSERT_EQ (block7->sideband ().height (), 7);
	auto block8 (store.block ().get (*transaction, state_open->hash ()));
	ASSERT_EQ (block8->sideband ().height (), 1);
	auto block9 (store.block ().get (*transaction, epoch->hash ()));
	ASSERT_EQ (block9->sideband ().height (), 2);
	auto block10 (store.block ().get (*transaction, epoch_open->hash ()));
	ASSERT_EQ (block10->sideband ().height (), 1);
	auto block11 (store.block ().get (*transaction, state_receive->hash ()));
	ASSERT_EQ (block11->sideband ().height (), 2);
	auto block12 (store.block ().get (*transaction, open->hash ()));
	ASSERT_EQ (block12->sideband ().height (), 1);
}

TEST (block_store, peers)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());

	nano::endpoint_key endpoint (boost::asio::ip::address_v6::any ().to_bytes (), 100);
	{
		auto transaction (store->tx_begin_write ());

		// Confirm that the store is empty
		ASSERT_FALSE (store->peer ().exists (*transaction, endpoint));
		ASSERT_EQ (store->peer ().count (*transaction), 0);

		// Add one
		store->peer ().put (*transaction, endpoint);
		ASSERT_TRUE (store->peer ().exists (*transaction, endpoint));
	}

	// Confirm that it can be found
	{
		auto transaction (store->tx_begin_read ());
		ASSERT_EQ (store->peer ().count (*transaction), 1);
	}

	// Add another one and check that it (and the existing one) can be found
	nano::endpoint_key endpoint1 (boost::asio::ip::address_v6::any ().to_bytes (), 101);
	{
		auto transaction (store->tx_begin_write ());
		store->peer ().put (*transaction, endpoint1);
		ASSERT_TRUE (store->peer ().exists (*transaction, endpoint1)); // Check new peer is here
		ASSERT_TRUE (store->peer ().exists (*transaction, endpoint)); // Check first peer is still here
	}

	{
		auto transaction (store->tx_begin_read ());
		ASSERT_EQ (store->peer ().count (*transaction), 2);
	}

	// Delete the first one
	{
		auto transaction (store->tx_begin_write ());
		store->peer ().del (*transaction, endpoint1);
		ASSERT_FALSE (store->peer ().exists (*transaction, endpoint1)); // Confirm it no longer exists
		ASSERT_TRUE (store->peer ().exists (*transaction, endpoint)); // Check first peer is still here
	}

	{
		auto transaction (store->tx_begin_read ());
		ASSERT_EQ (store->peer ().count (*transaction), 1);
	}

	// Delete original one
	{
		auto transaction (store->tx_begin_write ());
		store->peer ().del (*transaction, endpoint);
		ASSERT_FALSE (store->peer ().exists (*transaction, endpoint));
	}

	{
		auto transaction (store->tx_begin_read ());
		ASSERT_EQ (store->peer ().count (*transaction), 0);
	}
}

TEST (block_store, endpoint_key_byte_order)
{
	boost::asio::ip::address_v6 address (boost::asio::ip::make_address_v6 ("::ffff:127.0.0.1"));
	uint16_t port = 100;
	nano::endpoint_key endpoint_key (address.to_bytes (), port);

	std::vector<uint8_t> bytes;
	{
		nano::vectorstream stream (bytes);
		nano::write (stream, endpoint_key);
	}

	// This checks that the endpoint is serialized as expected, with a size
	// of 18 bytes (16 for ipv6 address and 2 for port), both in network byte order.
	ASSERT_EQ (bytes.size (), 18);
	ASSERT_EQ (bytes[10], 0xff);
	ASSERT_EQ (bytes[11], 0xff);
	ASSERT_EQ (bytes[12], 127);
	ASSERT_EQ (bytes[bytes.size () - 2], 0);
	ASSERT_EQ (bytes.back (), 100);

	// Deserialize the same stream bytes
	nano::bufferstream stream1 (bytes.data (), bytes.size ());
	nano::endpoint_key endpoint_key1;
	nano::read (stream1, endpoint_key1);

	// This should be in network bytes order
	ASSERT_EQ (address.to_bytes (), endpoint_key1.address_bytes ());

	// This should be in host byte order
	ASSERT_EQ (port, endpoint_key1.port ());
}

TEST (block_store, online_weight)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_FALSE (store->init_error ());
	{
		auto transaction (store->tx_begin_write ());
		ASSERT_EQ (0, store->online_weight ().count (*transaction));
		ASSERT_EQ (store->online_weight ().end (), store->online_weight ().begin (*transaction));
		ASSERT_EQ (store->online_weight ().end (), store->online_weight ().rbegin (*transaction));
		store->online_weight ().put (*transaction, 1, 2);
		store->online_weight ().put (*transaction, 3, 4);
	}
	{
		auto transaction (store->tx_begin_write ());
		ASSERT_EQ (2, store->online_weight ().count (*transaction));
		auto item (store->online_weight ().begin (*transaction));
		ASSERT_NE (store->online_weight ().end (), item);
		ASSERT_EQ (1, item->first);
		ASSERT_EQ (2, item->second.number ());
		auto item_last (store->online_weight ().rbegin (*transaction));
		ASSERT_NE (store->online_weight ().end (), item_last);
		ASSERT_EQ (3, item_last->first);
		ASSERT_EQ (4, item_last->second.number ());
		store->online_weight ().del (*transaction, 1);
		ASSERT_EQ (1, store->online_weight ().count (*transaction));
		ASSERT_EQ (store->online_weight ().begin (*transaction), store->online_weight ().rbegin (*transaction));
		store->online_weight ().del (*transaction, 3);
	}
	auto transaction (store->tx_begin_read ());
	ASSERT_EQ (0, store->online_weight ().count (*transaction));
	ASSERT_EQ (store->online_weight ().end (), store->online_weight ().begin (*transaction));
	ASSERT_EQ (store->online_weight ().end (), store->online_weight ().rbegin (*transaction));
}

TEST (block_store, pruned_blocks)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());

	nano::keypair key1;
	nano::block_builder builder;
	auto block1 = builder
				  .open ()
				  .source (0)
				  .representative (1)
				  .account (key1.pub)
				  .sign (key1.prv, key1.pub)
				  .work (0)
				  .build ();
	auto hash1 (block1->hash ());
	{
		auto transaction (store->tx_begin_write ());

		// Confirm that the store is empty
		ASSERT_FALSE (store->pruned ().exists (*transaction, hash1));
		ASSERT_EQ (store->pruned ().count (*transaction), 0);

		// Add one
		store->pruned ().put (*transaction, hash1);
		ASSERT_TRUE (store->pruned ().exists (*transaction, hash1));
	}

	// Confirm that it can be found
	ASSERT_EQ (store->pruned ().count (*store->tx_begin_read ()), 1);

	// Add another one and check that it (and the existing one) can be found
	auto block2 = builder
				  .open ()
				  .source (1)
				  .representative (2)
				  .account (key1.pub)
				  .sign (key1.prv, key1.pub)
				  .work (0)
				  .build ();
	block2->sideband_set ({});
	auto hash2 (block2->hash ());
	{
		auto transaction (store->tx_begin_write ());
		store->pruned ().put (*transaction, hash2);
		ASSERT_TRUE (store->pruned ().exists (*transaction, hash2)); // Check new pruned hash is here
		ASSERT_FALSE (store->block ().exists (*transaction, hash2));
		ASSERT_TRUE (store->pruned ().exists (*transaction, hash1)); // Check first pruned hash is still here
		ASSERT_FALSE (store->block ().exists (*transaction, hash1));
	}

	ASSERT_EQ (store->pruned ().count (*store->tx_begin_read ()), 2);

	// Delete the first one
	{
		auto transaction (store->tx_begin_write ());
		store->pruned ().del (*transaction, hash2);
		ASSERT_FALSE (store->pruned ().exists (*transaction, hash2)); // Confirm it no longer exists
		ASSERT_FALSE (store->block ().exists (*transaction, hash2)); // true for block_exists
		store->block ().put (*transaction, hash2, *block2); // Add corresponding block
		ASSERT_TRUE (store->block ().exists (*transaction, hash2));
		ASSERT_TRUE (store->pruned ().exists (*transaction, hash1)); // Check first pruned hash is still here
		ASSERT_FALSE (store->block ().exists (*transaction, hash1));
	}

	ASSERT_EQ (store->pruned ().count (*store->tx_begin_read ()), 1);

	// Delete original one
	{
		auto transaction (store->tx_begin_write ());
		store->pruned ().del (*transaction, hash1);
		ASSERT_FALSE (store->pruned ().exists (*transaction, hash1));
	}

	ASSERT_EQ (store->pruned ().count (*store->tx_begin_read ()), 0);
}

// Test various confirmation height values as well as clearing them
TEST (block_store, confirmation_height)
{
	auto path (nano::unique_path ());
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, path, nano::dev::constants);

	nano::account account1{};
	nano::account account2{ 1 };
	nano::account account3{ 2 };
	nano::block_hash cemented_frontier1 (3);
	nano::block_hash cemented_frontier2 (4);
	nano::block_hash cemented_frontier3 (5);
	{
		auto transaction (store->tx_begin_write ());
		store->confirmation_height ().put (*transaction, account1, { 500, cemented_frontier1 });
		store->confirmation_height ().put (*transaction, account2, { std::numeric_limits<uint64_t>::max (), cemented_frontier2 });
		store->confirmation_height ().put (*transaction, account3, { 10, cemented_frontier3 });

		nano::confirmation_height_info confirmation_height_info;
		ASSERT_FALSE (store->confirmation_height ().get (*transaction, account1, confirmation_height_info));
		ASSERT_EQ (confirmation_height_info.height (), 500);
		ASSERT_EQ (confirmation_height_info.frontier (), cemented_frontier1);
		ASSERT_FALSE (store->confirmation_height ().get (*transaction, account2, confirmation_height_info));
		ASSERT_EQ (confirmation_height_info.height (), std::numeric_limits<uint64_t>::max ());
		ASSERT_EQ (confirmation_height_info.frontier (), cemented_frontier2);
		ASSERT_FALSE (store->confirmation_height ().get (*transaction, account3, confirmation_height_info));
		ASSERT_EQ (confirmation_height_info.height (), 10);
		ASSERT_EQ (confirmation_height_info.frontier (), cemented_frontier3);

		// Check clearing of confirmation heights
		store->confirmation_height ().clear (*transaction);
	}
	auto transaction (store->tx_begin_read ());
	ASSERT_EQ (store->confirmation_height ().count (*transaction), 0);
	nano::confirmation_height_info confirmation_height_info;
	ASSERT_TRUE (store->confirmation_height ().get (*transaction, account1, confirmation_height_info));
	ASSERT_TRUE (store->confirmation_height ().get (*transaction, account2, confirmation_height_info));
	ASSERT_TRUE (store->confirmation_height ().get (*transaction, account3, confirmation_height_info));
}

// Test various confirmation height values as well as clearing them
TEST (block_store, final_vote)
{
	auto path (nano::unique_path ());
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, path, nano::dev::constants);

	{
		auto qualified_root = nano::dev::genesis->qualified_root ();
		auto transaction (store->tx_begin_write ());
		store->final_vote ().put (*transaction, qualified_root, nano::block_hash (2));
		ASSERT_EQ (store->final_vote ().count (*transaction), 1);
		store->final_vote ().clear (*transaction);
		ASSERT_EQ (store->final_vote ().count (*transaction), 0);
		store->final_vote ().put (*transaction, qualified_root, nano::block_hash (2));
		ASSERT_EQ (store->final_vote ().count (*transaction), 1);
		// Clearing with incorrect root shouldn't remove
		store->final_vote ().clear (*transaction, qualified_root.previous ());
		ASSERT_EQ (store->final_vote ().count (*transaction), 1);
		// Clearing with correct root should remove
		store->final_vote ().clear (*transaction, qualified_root.root ());
		ASSERT_EQ (store->final_vote ().count (*transaction), 0);
	}
}

// Ledger versions are not forward compatible
TEST (block_store, incompatible_version)
{
	auto path (nano::unique_path ());
	auto logger{ std::make_shared<nano::logger_mt> () };

	{
		auto store = nano::make_store (logger, path, nano::dev::constants);
		ASSERT_FALSE (store->init_error ());

		// Put version to an unreachable number so that it should always be incompatible
		auto transaction (store->tx_begin_write ());
		store->version ().put (*transaction, std::numeric_limits<int>::max ());
	}

	// Now try and read it, should give an error
	{
		auto store = nano::make_store (logger, path, nano::dev::constants, true);
		ASSERT_TRUE (store->init_error ());
	}
}

TEST (block_store, reset_renew_existing_transaction)
{
	auto logger{ std::make_shared<nano::logger_mt> () };

	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_TRUE (!store->init_error ());

	nano::keypair key1;
	nano::block_builder builder;
	auto block = builder
				 .open ()
				 .source (0)
				 .representative (1)
				 .account (1)
				 .sign (nano::keypair ().prv, 0)
				 .work (0)
				 .build ();
	block->sideband_set ({});
	auto hash1 (block->hash ());
	auto read_transaction = store->tx_begin_read ();

	// Block shouldn't exist yet
	auto block_non_existing (store->block ().get (*read_transaction, hash1));
	ASSERT_EQ (nullptr, block_non_existing);

	// Release resources for the transaction
	read_transaction->reset ();

	// Write the block
	{
		auto write_transaction (store->tx_begin_write ());
		store->block ().put (*write_transaction, hash1, *block);
	}

	read_transaction->renew ();

	// Block should exist now
	auto block_existing (store->block ().get (*read_transaction, hash1));
	ASSERT_NE (nullptr, block_existing);
}
