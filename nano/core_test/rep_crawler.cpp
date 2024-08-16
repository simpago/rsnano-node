#include <nano/lib/blocks.hpp>
#include <nano/lib/config.hpp>
#include <nano/lib/logging.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/repcrawler.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/test_common/chains.hpp>
#include <nano/test_common/network.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

using namespace std::chrono_literals;

// Test that nodes can track nodes that have rep weight for priority broadcasting
TEST (rep_crawler, rep_list)
{
	nano::test::system system;
	auto & node1 = *system.add_node ();
	auto & node2 = *system.add_node ();
	auto wallet_id1 = node1.wallets.first_wallet_id ();
	ASSERT_EQ (0, node2.rep_crawler.representative_count ());
	// Node #1 has a rep
	(void)node1.wallets.insert_adhoc (wallet_id1, nano::dev::genesis_key.prv);
	ASSERT_TIMELY_EQ (5s, node2.rep_crawler.representative_count (), 1);
	auto reps = node2.representative_register.representatives ();
	ASSERT_EQ (1, reps.size ());
	ASSERT_EQ (nano::dev::genesis_key.pub, reps[0].get_account ());
}

TEST (rep_crawler, rep_connection_close)
{
	nano::test::system system;
	auto & node1 = *system.add_node ();
	auto & node2 = *system.add_node ();
	// Add working representative (node 2)
	(void)node2.wallets.insert_adhoc (node2.wallets.first_wallet_id (), nano::dev::genesis_key.prv);
	ASSERT_TIMELY_EQ (10s, node1.rep_crawler.representative_count (), 1);
	system.stop_node (node2);
	// Remove representative with closed channel
	ASSERT_TIMELY_EQ (10s, node1.rep_crawler.representative_count (), 0);
}

