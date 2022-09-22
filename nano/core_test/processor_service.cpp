#include <nano/lib/stats.hpp>
#include <nano/lib/work.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/secure/store.hpp>
#include <nano/secure/utility.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

TEST (processor_service, bad_send_signature)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_FALSE (store->init_error ());
	nano::stat stats;
	nano::ledger ledger (*store, stats, nano::dev::constants);
	auto transaction (store->tx_begin_write ());
	store->initialize (*transaction, ledger.cache, ledger.constants);
	nano::work_pool pool{ nano::dev::network_params.network, std::numeric_limits<unsigned>::max () };
	nano::account_info info1;
	ASSERT_FALSE (store->account ().get (*transaction, nano::dev::genesis_key.pub, info1));
	nano::keypair key2;
	nano::block_builder builder;
	auto send = builder
				.send ()
				.previous (info1.head ())
				.destination (nano::dev::genesis_key.pub)
				.balance (50)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*pool.generate (info1.head ()))
				.build ();
	nano::signature sig{ send->block_signature () };
	sig.bytes[32] ^= 0x1;
	send->signature_set (sig);
	ASSERT_EQ (nano::process_result::bad_signature, ledger.process (*transaction, *send).code);
}

TEST (processor_service, bad_receive_signature)
{
	auto logger{ std::make_shared<nano::logger_mt> () };
	auto store = nano::make_store (logger, nano::unique_path (), nano::dev::constants);
	ASSERT_FALSE (store->init_error ());
	nano::stat stats;
	nano::ledger ledger (*store, stats, nano::dev::constants);
	auto transaction (store->tx_begin_write ());
	store->initialize (*transaction, ledger.cache, ledger.constants);
	nano::work_pool pool{ nano::dev::network_params.network, std::numeric_limits<unsigned>::max () };
	nano::account_info info1;
	ASSERT_FALSE (store->account ().get (*transaction, nano::dev::genesis_key.pub, info1));
	nano::block_builder builder;
	auto send = builder
				.send ()
				.previous (info1.head ())
				.destination (nano::dev::genesis_key.pub)
				.balance (50)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*pool.generate (info1.head ()))
				.build ();
	nano::block_hash hash1 (send->hash ());
	ASSERT_EQ (nano::process_result::progress, ledger.process (*transaction, *send).code);
	nano::account_info info2;
	ASSERT_FALSE (store->account ().get (*transaction, nano::dev::genesis_key.pub, info2));
	auto receive = builder
				   .receive ()
				   .previous (hash1)
				   .source (hash1)
				   .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				   .work (*pool.generate (hash1))
				   .build ();
	auto new_sig{ receive->block_signature () };
	new_sig.bytes[32] ^= 0x1;
	receive->signature_set (new_sig);
	ASSERT_EQ (nano::process_result::bad_signature, ledger.process (*transaction, *receive).code);
}
