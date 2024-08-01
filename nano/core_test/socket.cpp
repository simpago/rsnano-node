#include <nano/boost/asio/ip/address_v6.hpp>
#include <nano/boost/asio/ip/network_v6.hpp>
#include <nano/lib/thread_runner.hpp>
#include <nano/lib/threading.hpp>
#include <nano/node/inactive_node.hpp>
#include <nano/node/transport/tcp_listener.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <boost/asio/read.hpp>

#include <memory>
#include <vector>

using namespace std::chrono_literals;

TEST (socket, max_connections)
{
	// TODO implement again!
}

TEST (socket, max_connections_per_ip)
{
	// TODO implement again!
}

TEST (socket, max_connections_per_subnetwork)
{
	// TODO implement again!
}

TEST (socket, disabled_max_peers_per_ip)
{
	// TODO implement again!
}

TEST (socket, disconnection_of_silent_connections)
{
	// TODO implement again!
}

// Disabled, because it doesn't work with Tokio. The Test expects the async runtime to
// not do anything, so that the drop policy can trigger, but Tokio does make connections/sends
// and that prevents the drop. The test must be rewritten
TEST (DISABLED_socket, drop_policy)
{
	// TODO implement again!
}

/**
 * Check that the socket correctly handles a tcp_io_timeout during tcp connect
 * Steps:
 *   set timeout to one second
 *   do a tcp connect that will block for at least a few seconds at the tcp level
 *   check that the connect returns error and that the correct counters have been incremented
 *
 *   NOTE: it is possible that the O/S has tried to access the IP address 10.255.254.253 before
 *   and has it marked in the routing table as unroutable. In that case this test case will fail.
 *   If this test is run repeadetly the tests fails for this reason because the connection fails
 *   with "No route to host" error instead of a timeout.
 */
TEST (socket_timeout, DISABLED_connect)
{
	// TODO implement again!
}

TEST (socket_timeout, DISABLED_write)
{
	// TODO implement again!
}

TEST (socket_timeout, DISABLED_write_overlapped)
{
	// TODO implement again!
}
