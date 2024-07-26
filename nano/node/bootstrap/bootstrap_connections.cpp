#include <nano/node/bootstrap/bootstrap.hpp>
#include <nano/node/bootstrap/bootstrap_connections.hpp>
#include <nano/node/common.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/tcp.hpp>

#include <boost/format.hpp>

constexpr double nano::bootstrap_limits::bootstrap_connection_scale_target_blocks;
constexpr double nano::bootstrap_limits::bootstrap_minimum_blocks_per_sec;
constexpr double nano::bootstrap_limits::bootstrap_minimum_termination_time_sec;
constexpr unsigned nano::bootstrap_limits::bootstrap_max_new_connections;
constexpr unsigned nano::bootstrap_limits::requeued_pulls_processed_blocks_factor;

nano::bootstrap_client::bootstrap_client (rsnano::BootstrapClientHandle * handle_a) :
	handle{ handle_a }
{
}

nano::bootstrap_client::~bootstrap_client ()
{
	rsnano::rsn_bootstrap_client_destroy (handle);
}

unsigned nano::bootstrap_connections::target_connections (std::size_t pulls_remaining, std::size_t attempts_count) const
{
	return rsnano::rsn_bootstrap_connections_target_connections (handle, pulls_remaining, attempts_count);
}

nano::bootstrap_connections::bootstrap_connections (rsnano::BootstrapConnectionsHandle * handle) :
	handle{ handle }
{
}

nano::bootstrap_connections::~bootstrap_connections ()
{
	rsnano::rsn_bootstrap_connections_drop (handle);
}

void nano::bootstrap_connections::bootstrap_status (boost::property_tree::ptree & connections, std::size_t attempts_count)
{
	rsnano::rsn_bootstrap_connections_bootstrap_status (handle, &connections, attempts_count);
}

unsigned nano::bootstrap_connections::get_connections_count () const
{
	return rsnano::rsn_bootstrap_connections_connections_count (handle);
}
