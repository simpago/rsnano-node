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

TEST (rep_crawler, rep_weight)
{
	nano::test::system system;
	auto & node = *system.add_node ();
	auto & node1 = *system.add_node ();
	auto & node2 = *system.add_node ();
	auto & node3 = *system.add_node ();
	nano::keypair keypair1;
	nano::keypair keypair2;
	nano::block_builder builder;
	auto const amount_pr = node.minimum_principal_weight () + 100;
	auto const amount_not_pr = node.minimum_principal_weight () - 100;
	std::shared_ptr<nano::block> block1 = builder
										  .state ()
										  .account (nano::dev::genesis_key.pub)
										  .previous (nano::dev::genesis->hash ())
										  .representative (nano::dev::genesis_key.pub)
										  .balance (nano::dev::constants.genesis_amount - amount_not_pr)
										  .link (keypair1.pub)
										  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
										  .work (*system.work.generate (nano::dev::genesis->hash ()))
										  .build ();
	std::shared_ptr<nano::block> block2 = builder
										  .state ()
										  .account (keypair1.pub)
										  .previous (0)
										  .representative (keypair1.pub)
										  .balance (amount_not_pr)
										  .link (block1->hash ())
										  .sign (keypair1.prv, keypair1.pub)
										  .work (*system.work.generate (keypair1.pub))
										  .build ();
	std::shared_ptr<nano::block> block3 = builder
										  .state ()
										  .account (nano::dev::genesis_key.pub)
										  .previous (block1->hash ())
										  .representative (nano::dev::genesis_key.pub)
										  .balance (nano::dev::constants.genesis_amount - amount_not_pr - amount_pr)
										  .link (keypair2.pub)
										  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
										  .work (*system.work.generate (block1->hash ()))
										  .build ();
	std::shared_ptr<nano::block> block4 = builder
										  .state ()
										  .account (keypair2.pub)
										  .previous (0)
										  .representative (keypair2.pub)
										  .balance (amount_pr)
										  .link (block3->hash ())
										  .sign (keypair2.prv, keypair2.pub)
										  .work (*system.work.generate (keypair2.pub))
										  .build ();
	ASSERT_TRUE (nano::test::process (node, { block1, block2, block3, block4 }));
	ASSERT_TRUE (nano::test::process (node1, { block1, block2, block3, block4 }));
	ASSERT_TRUE (nano::test::process (node2, { block1, block2, block3, block4 }));
	ASSERT_TRUE (nano::test::process (node3, { block1, block2, block3, block4 }));
	ASSERT_TRUE (node.representative_register.representatives (1).empty ());

	ASSERT_TIMELY (5s, node.network->size () == 3);
	auto channel1 = node.network->find_node_id (node1.node_id.pub);
	auto channel2 = node.network->find_node_id (node2.node_id.pub);
	auto channel3 = node.network->find_node_id (node3.node_id.pub);
	ASSERT_NE (nullptr, channel1);
	ASSERT_NE (nullptr, channel2);
	ASSERT_NE (nullptr, channel3);
	auto vote0 = std::make_shared<nano::vote> (nano::dev::genesis_key.pub, nano::dev::genesis_key.prv, 0, 0, std::vector<nano::block_hash>{ nano::dev::genesis->hash () });
	auto vote1 = std::make_shared<nano::vote> (keypair1.pub, keypair1.prv, 0, 0, std::vector<nano::block_hash>{ nano::dev::genesis->hash () });
	auto vote2 = std::make_shared<nano::vote> (keypair2.pub, keypair2.prv, 0, 0, std::vector<nano::block_hash>{ nano::dev::genesis->hash () });
	node.rep_crawler.force_process (vote0, channel1);
	node.rep_crawler.force_process (vote1, channel2);
	node.rep_crawler.force_process (vote2, channel3);
	ASSERT_TIMELY_EQ (5s, node.representative_register.representative_count (), 2);
	// Make sure we get the rep with the most weight first
	auto reps = node.representative_register.representatives (1);
	ASSERT_EQ (1, reps.size ());
	ASSERT_EQ (node.balance (nano::dev::genesis_key.pub), node.ledger.weight (reps[0].get_account ()));
	ASSERT_EQ (nano::dev::genesis_key.pub, reps[0].get_account ());
	ASSERT_EQ (channel1->channel_id (), reps[0].channel_id ());
	ASSERT_TRUE (node.rep_crawler.is_pr (channel1));
	ASSERT_FALSE (node.rep_crawler.is_pr (channel2));
	ASSERT_TRUE (node.rep_crawler.is_pr (channel3));
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

// This test checks that if a block is in the recently_confirmed list then the repcrawler will not send a request for it.
// The behaviour of this test previously was the opposite, that the repcrawler eventually send out such a block and deleted the block
// from the recently confirmed list to try to make ammends for sending it, which is bad behaviour.
// In the long term, we should have a better way to check for reps and this test should become redundant
TEST (rep_crawler, recently_confirmed)
{
	nano::test::system system (1);
	auto & node1 (*system.nodes[0]);
	ASSERT_EQ (1, node1.ledger.block_count ());
	auto const block = nano::dev::genesis;
	node1.active.insert_recently_confirmed (block);
	auto & node2 (*system.add_node ());
	auto wallet_id2 = node2.wallets.first_wallet_id ();
	(void)node2.wallets.insert_adhoc (wallet_id2, nano::dev::genesis_key.prv);
	auto channel = node1.network->find_node_id (node2.get_node_id ());
	ASSERT_NE (nullptr, channel);
	node1.rep_crawler.query (channel); // this query should be dropped due to the recently_confirmed entry
	ASSERT_ALWAYS_EQ (0.5s, node1.rep_crawler.representative_count (), 0);
}
