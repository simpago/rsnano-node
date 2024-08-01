#include <nano/node/transport/tcp.hpp>
#include <nano/test_common/network.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <chrono>
#include <memory>

using namespace std::chrono_literals;

TEST (peer_container, empty_peers)
{
	nano::test::system system (1);
	nano::network & network (*system.nodes[0]->network);
	system.nodes[0]->network->cleanup (std::chrono::system_clock::now ());
	ASSERT_EQ (0, network.size ());
}

// Test a node cannot connect to its own endpoint.
TEST (peer_container, no_self_incoming)
{
	nano::test::system system{ 1 };
	auto & node = *system.nodes[0];
	node.connect (node.network->endpoint ());
	auto error = system.poll_until_true (2s, [&node] {
		auto result = node.network->tcp_channels->find_channel (nano::transport::map_endpoint_to_tcp (node.network->endpoint ()));
		return result != nullptr;
	});
	ASSERT_TRUE (error);
	ASSERT_TRUE (system.nodes[0]->network->empty ());
}

// Tests the function network not_a_peer function used by the nano::transport::tcp_channels.insert ()
TEST (peer_container, reserved_ip_is_not_a_peer)
{
	nano::test::system system{ 1 };
	auto not_a_peer = [&node = system.nodes[0]] (nano::endpoint endpoint_a) -> bool {
		return node->network->tcp_channels->not_a_peer (endpoint_a, true);
	};

	// The return value as true means an error because the IP address is for reserved use
	ASSERT_TRUE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0x00000001)), 10000))));
	ASSERT_TRUE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0xc0000201)), 10000))));
	ASSERT_TRUE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0xc6336401)), 10000))));
	ASSERT_TRUE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0xcb007101)), 10000))));
	ASSERT_TRUE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0xe9fc0001)), 10000))));
	ASSERT_TRUE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0xf0000001)), 10000))));
	ASSERT_TRUE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0xffffffff)), 10000))));

	// Test with a valid IP address
	ASSERT_FALSE (not_a_peer (nano::transport::map_endpoint_to_v6 (nano::endpoint (boost::asio::ip::address (boost::asio::ip::address_v4 (0x08080808)), 10000))));
}

// Test the TCP channel cleanup function works properly. It is used to remove peers that are not
// exchanging messages after a while.
TEST (peer_container, tcp_channel_cleanup_works)
{
	// TODO reimplement in Rust
}

TEST (channels, fill_random_clear)
{
	nano::test::system system (1);
	std::array<nano::endpoint, 8> target;
	std::fill (target.begin (), target.end (), nano::endpoint (boost::asio::ip::address_v6::loopback (), 10000));
	system.nodes[0]->network->tcp_channels->random_fill (target);
	ASSERT_TRUE (std::all_of (target.begin (), target.end (), [] (nano::endpoint const & endpoint_a) { return endpoint_a == nano::endpoint (boost::asio::ip::address_v6::any (), 0); }));
}

// Test all targets get replaced by random_fill
TEST (channels, fill_random_full)
{
	nano::test::system system{ 1 };

	// create 8 peer nodes so that the random_fill is completely filled with real connection data
	for (int i = 0; i < 8; ++i)
	{
		auto outer_node = nano::test::add_outer_node (system);
		nano::test::establish_tcp (system, *system.nodes[0], outer_node->network->endpoint ());
	}
	ASSERT_TIMELY_EQ (5s, 8, system.nodes[0]->network->tcp_channels->size ());

	// create an array of 8 endpoints with a known filler value
	auto filler_endpoint = nano::endpoint (boost::asio::ip::address_v6::loopback (), 10000);
	std::array<nano::endpoint, 8> target;
	std::fill (target.begin (), target.end (), filler_endpoint);

	// random fill target array with endpoints taken from the network connections
	system.nodes[0]->network->tcp_channels->random_fill (target);

	// check that all element in target got overwritten
	auto is_filler = [&filler_endpoint] (nano::endpoint const & endpoint_a) {
		return endpoint_a == filler_endpoint;
	};
	ASSERT_TRUE (std::none_of (target.begin (), target.end (), is_filler));
}

// Test only the known channels are filled
TEST (channels, fill_random_part)
{
	nano::test::system system{ 1 };
	std::array<nano::endpoint, 8> target;
	std::size_t half = target.size () / 2;
	for (std::size_t i = 0; i < half; ++i)
	{
		auto outer_node = nano::test::add_outer_node (system);
		nano::test::establish_tcp (system, *system.nodes[0], outer_node->network->endpoint ());
	}
	ASSERT_EQ (half, system.nodes[0]->network->tcp_channels->size ());
	std::fill (target.begin (), target.end (), nano::endpoint (boost::asio::ip::address_v6::loopback (), 10000));
	system.nodes[0]->network->tcp_channels->random_fill (target);
	ASSERT_TRUE (std::none_of (target.begin (), target.begin () + half, [] (nano::endpoint const & endpoint_a) { return endpoint_a == nano::endpoint (boost::asio::ip::address_v6::loopback (), 10000); }));
	ASSERT_TRUE (std::none_of (target.begin (), target.begin () + half, [] (nano::endpoint const & endpoint_a) { return endpoint_a == nano::endpoint (boost::asio::ip::address_v6::loopback (), 0); }));
	ASSERT_TRUE (std::all_of (target.begin () + half, target.end (), [] (nano::endpoint const & endpoint_a) { return endpoint_a == nano::endpoint (boost::asio::ip::address_v6::any (), 0); }));
}

TEST (peer_container, list_fanout)
{
	// TODO reimplement in Rust
}

// This test is similar to network.filter_invalid_version_using with the difference that
// this one checks for the channel's connection to get stopped when an incoming message
// is from an outdated node version.
//
// Disabled because there is currently no way to use different network version
TEST (DISABLED_peer_container, depeer_on_outdated_version)
{
	// TODO reimplement in Rust
}
