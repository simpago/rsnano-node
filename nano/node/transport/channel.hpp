#pragma once

#include <nano/lib/locks.hpp>
#include <nano/lib/stats.hpp>
#include <nano/node/bandwidth_limiter.hpp>
#include <nano/node/common.hpp>
#include <nano/node/messages.hpp>

#include <boost/asio/ip/network_v6.hpp>

#include <chrono>
#include <cstdint>

namespace rsnano
{
class BandwidthLimiterHandle;
class ChannelHandle;
}

namespace nano::transport
{
enum class transport_type : uint8_t
{
	undefined = 0,
	tcp = 1,
};

class channel
{
public:
	channel (rsnano::ChannelHandle * handle_a);
	channel (nano::transport::channel const &) = delete;
	virtual ~channel ();

	void close ();

	virtual std::string to_string () const = 0;
	virtual nano::endpoint get_remote_endpoint () const = 0;
	virtual nano::tcp_endpoint get_tcp_remote_endpoint () const = 0;
	virtual nano::tcp_endpoint get_peering_endpoint () const;
	virtual nano::transport::transport_type get_type () const = 0;

	std::chrono::system_clock::time_point get_last_packet_received () const;

	std::chrono::system_clock::time_point get_last_packet_sent () const;
	void set_last_packet_sent (std::chrono::system_clock::time_point time);

	boost::optional<nano::account> get_node_id_optional () const;
	nano::account get_node_id () const;
	void set_node_id (nano::account node_id_a);

	size_t channel_id () const;

	virtual uint8_t get_network_version () const = 0;
	rsnano::ChannelHandle * handle;
};
}
