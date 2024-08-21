#pragma once

#include "nano/lib/rsnano.hpp"
#include <nano/node/common.hpp>
#include <nano/node/transport/transport.hpp>

#include <chrono>
#include <cstddef>
#include <cstdint>
#include <memory>

namespace nano
{
class bootstrap_server;
class node_config;
class node_flags;
class network;
class logger;

namespace transport
{
	class tcp_channels final : public std::enable_shared_from_this<tcp_channels>
	{
	public:
		explicit tcp_channels (rsnano::TcpChannelsHandle * handle, rsnano::NetworkFilterHandle * filter_handle);
		tcp_channels (nano::transport::tcp_channels const &) = delete;
		~tcp_channels ();

		std::size_t size () const;

		uint16_t port () const;

		std::shared_ptr<nano::network_filter> publish_filter;

	public:
		rsnano::TcpChannelsHandle * handle;

		friend class network_peer_max_tcp_attempts_subnetwork_Test;
	};
} // namespace transport
} // namespace nano
