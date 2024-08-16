#pragma once

#include "nano/lib/rsnano.hpp"
#include "nano/node/election.hpp"

#include <nano/node/common.hpp>
#include <nano/node/transport/channel.hpp>
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
class syn_cookies;
class logger;

namespace transport
{
	class tcp_channels;

	class channel_tcp : public nano::transport::channel
	{
		friend class nano::transport::tcp_channels;

	public:
		channel_tcp (rsnano::ChannelHandle * handle_a) :
			channel{ handle_a } {};

		uint8_t get_network_version () const override;

		std::string to_string () const override;

		nano::endpoint get_remote_endpoint () const override
		{
			return nano::transport::map_tcp_to_endpoint (get_tcp_remote_endpoint ());
		}

		nano::tcp_endpoint get_tcp_remote_endpoint () const override;
		nano::transport::transport_type get_type () const override
		{
			return nano::transport::transport_type::tcp;
		}
	};

	class tcp_channels final : public std::enable_shared_from_this<tcp_channels>
	{
		friend class nano::transport::channel_tcp;

	public:
		explicit tcp_channels (rsnano::TcpChannelsHandle * handle, rsnano::NetworkFilterHandle * filter_handle);
		tcp_channels (nano::transport::tcp_channels const &) = delete;
		~tcp_channels ();

		std::size_t size () const;
		float size_sqrt () const;
		// Desired fanout for a given scale
		std::size_t fanout (float scale = 1.0f) const;
		std::vector<std::shared_ptr<nano::transport::channel>> random_channels (std::size_t, uint8_t = 0) const;
		std::shared_ptr<nano::transport::channel_tcp> find_node_id (nano::account const &);
		bool not_a_peer (nano::endpoint const &, bool);
		void purge (std::chrono::system_clock::time_point const & cutoff_deadline);
		std::deque<std::shared_ptr<nano::transport::channel>> list (std::size_t max_count = 0, uint8_t = 0);
		std::deque<std::shared_ptr<nano::transport::channel>> random_fanout (float scale = 1.0f);

		void random_fill (std::array<nano::endpoint, 8> &) const;
		uint16_t port () const;
		std::size_t get_next_channel_id ();

		std::shared_ptr<nano::network_filter> publish_filter;

	public:
		rsnano::TcpChannelsHandle * handle;

		friend class network_peer_max_tcp_attempts_subnetwork_Test;
	};

	std::shared_ptr<nano::transport::channel> channel_handle_to_channel (rsnano::ChannelHandle * handle);
} // namespace transport
} // namespace nano
