#pragma once

#include "nano/lib/rsnano.hpp"

#include <nano/node/bootstrap/bootstrap_bulk_pull.hpp>
#include <nano/node/common.hpp>
#include <nano/node/transport/socket.hpp>

namespace nano
{
class node;
class bootstrap_attempt;
class bootstrap_connections;
class pull_info;
class bootstrap_initiator;

/**
 * Owns the client side of the bootstrap connection.
 */
class bootstrap_client final : public std::enable_shared_from_this<bootstrap_client>
{
public:
	bootstrap_client (rsnano::BootstrapClientHandle * handle_a);
	~bootstrap_client ();
	rsnano::BootstrapClientHandle * handle;
};

/**
 * Container for bootstrap_client objects. Owned by bootstrap_initiator which pools open connections and makes them available
 * for use by different bootstrap sessions.
 */
class bootstrap_connections final : public std::enable_shared_from_this<bootstrap_connections>
{
public:
	bootstrap_connections (rsnano::BootstrapConnectionsHandle * handle);
	bootstrap_connections (bootstrap_connections const &) = delete;
	~bootstrap_connections ();
	unsigned target_connections (std::size_t pulls_remaining, std::size_t attempts_count) const;
	void bootstrap_status (boost::property_tree::ptree & tree, std::size_t attempts_count);
	unsigned get_connections_count () const;

	rsnano::BootstrapConnectionsHandle * handle{ nullptr };
};
}
