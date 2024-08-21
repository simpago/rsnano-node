#include <nano/lib/blocks.hpp>
#include <nano/lib/config.hpp>
#include <nano/node/network.hpp>
#include <nano/node/nodeconfig.hpp>
#include <nano/node/scheduler/component.hpp>
#include <nano/node/scheduler/priority.hpp>
#include <nano/node/transport/tcp_listener.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/store/component.hpp>
#include <nano/test_common/network.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <boost/iostreams/stream_buffer.hpp>
#include <boost/range/join.hpp>
#include <boost/thread.hpp>

#include <thread>

using namespace std::chrono_literals;

TEST (network, tcp_connection)
{
	nano::test::system system;
	boost::asio::ip::tcp::acceptor acceptor (system.async_rt.io_ctx);
	auto port = system.get_available_port ();
	boost::asio::ip::tcp::endpoint endpoint (boost::asio::ip::address_v4::any (), port);
	acceptor.open (endpoint.protocol ());
	acceptor.set_option (boost::asio::ip::tcp::acceptor::reuse_address (true));
	acceptor.bind (endpoint);
	acceptor.listen ();
	boost::asio::ip::tcp::socket incoming (system.async_rt.io_ctx);
	std::atomic<bool> done1 (false);
	std::string message1;
	acceptor.async_accept (incoming, [&done1, &message1] (boost::system::error_code const & ec_a) {
		if (ec_a)
		{
			message1 = ec_a.message ();
			std::cerr << message1;
		}
		done1 = true;
	});
	boost::asio::ip::tcp::socket connector (system.async_rt.io_ctx);
	std::atomic<bool> done2 (false);
	std::string message2;
	connector.async_connect (boost::asio::ip::tcp::endpoint (boost::asio::ip::address_v4::loopback (), acceptor.local_endpoint ().port ()),
	[&done2, &message2] (boost::system::error_code const & ec_a) {
		if (ec_a)
		{
			message2 = ec_a.message ();
			std::cerr << message2;
		}
		done2 = true;
	});
	ASSERT_TIMELY (5s, done1 && done2);
	ASSERT_EQ (0, message1.size ());
	ASSERT_EQ (0, message2.size ());
}

TEST (network, construction_with_specified_port)
{
	nano::test::system system{};
	auto const port = nano::test::speculatively_choose_a_free_tcp_bind_port ();
	ASSERT_NE (port, 0);
	auto const node = system.add_node (nano::node_config{ port });
	EXPECT_EQ (port, node->network->tcp_channels->port ());
	EXPECT_EQ (port, node->network->endpoint ().port ());
	EXPECT_EQ (port, node->tcp_listener->endpoint ().port ());
}

TEST (network, construction_without_specified_port)
{
	nano::test::system system{};
	auto const node = system.add_node ();
	auto const port = node->network->tcp_channels->port ();
	EXPECT_NE (0, port);
	EXPECT_EQ (port, node->network->endpoint ().port ());
	EXPECT_EQ (port, node->tcp_listener->endpoint ().port ());
}

// Disabled, because it is flakey with Tokio
TEST (DISABLED_network, send_node_id_handshake_tcp)
{
	// TODO reimplement in Rust
}

TEST (network, multi_keepalive)
{
	nano::test::system system (1);
	auto node0 = system.nodes[0];
	ASSERT_EQ (0, node0->network->size ());
	auto node1 (std::make_shared<nano::node> (system.async_rt, system.get_available_port (), nano::unique_path (), system.work));
	ASSERT_FALSE (node1->init_error ());
	node1->start ();
	system.nodes.push_back (node1);
	ASSERT_EQ (0, node1->network->size ());
	ASSERT_EQ (0, node0->network->size ());
	node1->connect (node0->network->endpoint ());
	ASSERT_TIMELY (10s, node0->network->size () == 1 && node0->stats->count (nano::stat::type::message, nano::stat::detail::keepalive) >= 1);
	auto node2 (std::make_shared<nano::node> (system.async_rt, system.get_available_port (), nano::unique_path (), system.work));
	ASSERT_FALSE (node2->init_error ());
	node2->start ();
	system.nodes.push_back (node2);
	node2->connect (node0->network->endpoint ());
	// ASSERT_TIMELY (10s, node1->network->size () == 2 && node0->network->size () == 2 && node2->network->size () == 2 && node0->stats->count (nano::stat::type::message, nano::stat::detail::keepalive) >= 2);
	std::this_thread::sleep_for (10s);
	std::cout << "node0: " << node0->network->size () << ", node1: " << node1->network->size () << ", node2: " << node2->network->size () << std::endl;
}

TEST (network, send_valid_confirm_ack)
{
	nano::node_flags node_flags;
	nano::test::system system (2, node_flags);
	auto & node1 (*system.nodes[0]);
	auto & node2 (*system.nodes[1]);
	auto wallet_id1 = node1.wallets.first_wallet_id ();
	auto wallet_id2 = node2.wallets.first_wallet_id ();
	nano::keypair key2;
	(void)node1.wallets.insert_adhoc (wallet_id1, nano::dev::genesis_key.prv);
	(void)node2.wallets.insert_adhoc (wallet_id2, key2.prv);
	nano::block_hash latest1 (node1.latest (nano::dev::genesis_key.pub));
	nano::block_builder builder;
	auto block2 = builder
				  .send ()
				  .previous (latest1)
				  .destination (key2.pub)
				  .balance (50)
				  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				  .work (*system.work.generate (latest1))
				  .build ();
	nano::block_hash latest2 (node2.latest (nano::dev::genesis_key.pub));
	node1.process_active (std::make_shared<nano::send_block> (*block2));
	// Keep polling until latest block changes
	ASSERT_TIMELY (10s, node2.latest (nano::dev::genesis_key.pub) != latest2);
	// Make sure the balance has decreased after processing the block.
	ASSERT_EQ (50, node2.balance (nano::dev::genesis_key.pub));
}

TEST (network, send_valid_publish)
{
	nano::node_flags node_flags;
	nano::test::system system (2, node_flags);
	auto & node1 (*system.nodes[0]);
	auto & node2 (*system.nodes[1]);
	auto wallet_id1 = node1.wallets.first_wallet_id ();
	auto wallet_id2 = node2.wallets.first_wallet_id ();
	node1.bootstrap_initiator.stop ();
	node2.bootstrap_initiator.stop ();
	(void)node1.wallets.insert_adhoc (wallet_id1, nano::dev::genesis_key.prv);
	nano::keypair key2;
	(void)node2.wallets.insert_adhoc (wallet_id2, key2.prv);
	nano::block_hash latest1 (node1.latest (nano::dev::genesis_key.pub));
	nano::block_builder builder;
	auto block2 = builder
				  .send ()
				  .previous (latest1)
				  .destination (key2.pub)
				  .balance (50)
				  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				  .work (*system.work.generate (latest1))
				  .build ();
	auto hash2 (block2->hash ());
	nano::block_hash latest2 (node2.latest (nano::dev::genesis_key.pub));
	node2.process_active (std::make_shared<nano::send_block> (*block2));
	ASSERT_TIMELY (10s, node1.stats->count (nano::stat::type::message, nano::stat::detail::publish, nano::stat::dir::in) != 0);
	ASSERT_NE (hash2, latest2);
	ASSERT_TIMELY (10s, node2.latest (nano::dev::genesis_key.pub) != latest2);
	ASSERT_EQ (50, node2.balance (nano::dev::genesis_key.pub));
}

TEST (receivable_processor, send_with_receive)
{
	nano::node_flags node_flags;
	nano::test::system system (2, node_flags);
	auto & node1 (*system.nodes[0]);
	auto & node2 (*system.nodes[1]);
	auto wallet_id1 = node1.wallets.first_wallet_id ();
	auto wallet_id2 = node2.wallets.first_wallet_id ();
	auto amount (std::numeric_limits<nano::uint128_t>::max ());
	nano::keypair key2;
	(void)node1.wallets.insert_adhoc (wallet_id1, nano::dev::genesis_key.prv);
	nano::block_hash latest1 (node1.latest (nano::dev::genesis_key.pub));
	nano::block_builder builder;
	auto block1 = builder
				  .send ()
				  .previous (latest1)
				  .destination (key2.pub)
				  .balance (amount - node1.config->receive_minimum.number ())
				  .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				  .work (*system.work.generate (latest1))
				  .build ();
	ASSERT_EQ (amount, node1.balance (nano::dev::genesis_key.pub));
	ASSERT_EQ (0, node1.balance (key2.pub));
	ASSERT_EQ (amount, node2.balance (nano::dev::genesis_key.pub));
	ASSERT_EQ (0, node2.balance (key2.pub));
	node1.process_active (block1);
	ASSERT_TIMELY (5s, nano::test::exists (node1, { block1 }));
	node2.process_active (block1);
	ASSERT_TIMELY (5s, nano::test::exists (node2, { block1 }));
	ASSERT_EQ (amount - node1.config->receive_minimum.number (), node1.balance (nano::dev::genesis_key.pub));
	ASSERT_EQ (0, node1.balance (key2.pub));
	ASSERT_EQ (amount - node1.config->receive_minimum.number (), node2.balance (nano::dev::genesis_key.pub));
	ASSERT_EQ (0, node2.balance (key2.pub));
	(void)node2.wallets.insert_adhoc (wallet_id2, key2.prv);
	ASSERT_TIMELY (10s, node1.balance (key2.pub) == node1.config->receive_minimum.number () && node2.balance (key2.pub) == node1.config->receive_minimum.number ());
	ASSERT_EQ (amount - node1.config->receive_minimum.number (), node1.balance (nano::dev::genesis_key.pub));
	ASSERT_EQ (node1.config->receive_minimum.number (), node1.balance (key2.pub));
	ASSERT_EQ (amount - node1.config->receive_minimum.number (), node2.balance (nano::dev::genesis_key.pub));
	ASSERT_EQ (node1.config->receive_minimum.number (), node2.balance (key2.pub));
}

TEST (network, receive_weight_change)
{
	nano::test::system system (2);
	auto node1 = system.nodes[0];
	auto node2 = system.nodes[1];
	auto wallet_id1 = node1->wallets.first_wallet_id ();
	auto wallet_id2 = node2->wallets.first_wallet_id ();
	(void)node1->wallets.insert_adhoc (wallet_id1, nano::dev::genesis_key.prv);
	nano::keypair key2;
	(void)node2->wallets.insert_adhoc (wallet_id2, key2.prv);
	(void)node2->wallets.set_representative (wallet_id2, key2.pub);
	ASSERT_NE (nullptr, node1->wallets.send_action (wallet_id1, nano::dev::genesis_key.pub, key2.pub, system.nodes[0]->config->receive_minimum.number ()));
	ASSERT_TIMELY (10s, std::all_of (system.nodes.begin (), system.nodes.end (), [&] (std::shared_ptr<nano::node> const & node_a) { return node_a->weight (key2.pub) == system.nodes[0]->config->receive_minimum.number (); }));
}

TEST (parse_endpoint, valid)
{
	std::string string ("::1:24000");
	nano::endpoint endpoint;
	ASSERT_FALSE (nano::parse_endpoint (string, endpoint));
	ASSERT_EQ (boost::asio::ip::address_v6::loopback (), endpoint.address ());
	ASSERT_EQ (24000, endpoint.port ());
}

TEST (parse_endpoint, invalid_port)
{
	std::string string ("::1:24a00");
	nano::endpoint endpoint;
	ASSERT_TRUE (nano::parse_endpoint (string, endpoint));
}

TEST (parse_endpoint, invalid_address)
{
	std::string string ("::q:24000");
	nano::endpoint endpoint;
	ASSERT_TRUE (nano::parse_endpoint (string, endpoint));
}

TEST (parse_endpoint, no_address)
{
	std::string string (":24000");
	nano::endpoint endpoint;
	ASSERT_TRUE (nano::parse_endpoint (string, endpoint));
}

TEST (parse_endpoint, no_port)
{
	std::string string ("::1:");
	nano::endpoint endpoint;
	ASSERT_TRUE (nano::parse_endpoint (string, endpoint));
}

TEST (parse_endpoint, no_colon)
{
	std::string string ("::1");
	nano::endpoint endpoint;
	ASSERT_TRUE (nano::parse_endpoint (string, endpoint));
}

TEST (network, ipv6)
{
	boost::asio::ip::address_v6 address (boost::asio::ip::make_address_v6 ("::ffff:127.0.0.1"));
	ASSERT_TRUE (address.is_v4_mapped ());
	nano::endpoint endpoint1 (address, 16384);
	std::vector<uint8_t> bytes1;
	{
		nano::vectorstream stream (bytes1);
		nano::write (stream, address.to_bytes ());
	}
	ASSERT_EQ (16, bytes1.size ());
	for (auto i (bytes1.begin ()), n (bytes1.begin () + 10); i != n; ++i)
	{
		ASSERT_EQ (0, *i);
	}
	ASSERT_EQ (0xff, bytes1[10]);
	ASSERT_EQ (0xff, bytes1[11]);
	std::array<uint8_t, 16> bytes2;
	nano::bufferstream stream (bytes1.data (), bytes1.size ());
	auto error (nano::try_read (stream, bytes2));
	ASSERT_FALSE (error);
	nano::endpoint endpoint2 (boost::asio::ip::address_v6 (bytes2), 16384);
	ASSERT_EQ (endpoint1, endpoint2);
}

TEST (network, ipv6_from_ipv4)
{
	nano::endpoint endpoint1 (boost::asio::ip::address_v4::loopback (), 16000);
	ASSERT_TRUE (endpoint1.address ().is_v4 ());
	nano::endpoint endpoint2 (boost::asio::ip::address_v6::v4_mapped (endpoint1.address ().to_v4 ()), 16000);
	ASSERT_TRUE (endpoint2.address ().is_v6 ());
}

// Test disabled because it's failing intermittently.
// PR in which it got disabled: https://github.com/nanocurrency/nano-node/pull/3611
// Issue for investigating it: https://github.com/nanocurrency/nano-node/issues/3615
TEST (tcp_listener, DISABLED_tcp_listener_timeout_empty)
{
	// TODO reimplement in Rust
}

TEST (tcp_listener, tcp_listener_timeout_node_id_handshake)
{
	// TODO reimplement in Rust
}

// Test disabled because it's failing repeatedly for Windows + LMDB.
// PR in which it got disabled: https://github.com/nanocurrency/nano-node/pull/3622
// Issue for investigating it: https://github.com/nanocurrency/nano-node/issues/3621
#ifndef _WIN32
// Disabled, because it does not work with Tokio, because Tokio executes the async requests
// and this test assumes that the async runtime doesn't poll. Test must be rewritten!
TEST (DISABLED_network, peer_max_tcp_attempts)
{
	// TODO reimplement in Rust
}
#endif

TEST (network, peer_max_tcp_attempts_subnetwork)
{
	// TODO reimplement in Rust
}

TEST (network, tcp_no_accept_excluded_peers)
{
	// TODO reimplement in Rust
}

// Ensure the network filters messages with the incorrect magic number
// Disabled, because there is currently no way to send messages with a given network id
TEST (DISABLED_network, filter_invalid_network_bytes)
{
	// TODO reimplement in Rust
}

// Ensure the network filters messages with the incorrect minimum version
// Disabled, because there is currently no way to send messages with a given version
TEST (DISABLED_network, filter_invalid_version_using)
{
	// TODO reimplement in Rust
}

/*
 * Tests that channel and channel container removes channels with dead local sockets
 */
TEST (network, purge_dead_channel_outgoing)
{
	// TODO reimplement in Rust
}

/*
 * Tests that channel and channel container removes channels with dead remote sockets
 */
TEST (network, purge_dead_channel_incoming)
{
	// TODO reimplement in Rust
}
