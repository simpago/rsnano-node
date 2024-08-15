#include "nano/lib/rsnano.hpp"
#include "nano/lib/rsnanoutils.hpp"
#include "nano/node/messages.hpp"
#include "nano/node/transport/channel.hpp"
#include "nano/secure/network_filter.hpp"

#include <nano/crypto_lib/random_pool_shuffle.hpp>
#include <nano/lib/config.hpp>
#include <nano/lib/stats.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/tcp.hpp>

#include <boost/format.hpp>

#include <chrono>
#include <cstddef>
#include <cstdint>
#include <iterator>
#include <memory>
#include <stdexcept>
#include <unordered_set>

/*
 * channel_tcp
 */

namespace
{
std::vector<std::shared_ptr<nano::transport::channel>> into_channel_vector (rsnano::ChannelListHandle * list_handle)
{
	auto len = rsnano::rsn_channel_list_len (list_handle);
	std::vector<std::shared_ptr<nano::transport::channel>> result;
	result.reserve (len);
	for (auto i = 0; i < len; ++i)
	{
		auto channel_handle = rsnano::rsn_channel_list_get (list_handle, i);
		result.push_back (std::make_shared<nano::transport::channel_tcp> (channel_handle));
	}
	rsnano::rsn_channel_list_destroy (list_handle);
	return result;
}
}

uint8_t nano::transport::channel_tcp::get_network_version () const
{
	return rsnano::rsn_channel_tcp_network_version (handle);
}

nano::tcp_endpoint nano::transport::channel_tcp::get_tcp_remote_endpoint () const
{
	rsnano::EndpointDto ep_dto{};
	rsnano::rsn_channel_tcp_remote_endpoint (handle, &ep_dto);
	return rsnano::dto_to_endpoint (ep_dto);
}

std::string nano::transport::channel_tcp::to_string () const
{
	return boost::str (boost::format ("%1%") % get_tcp_remote_endpoint ());
}

/*
 * tcp_channels
 */

nano::transport::tcp_channels::tcp_channels (rsnano::TcpChannelsHandle * handle, rsnano::NetworkFilterHandle * filter_handle) :
	handle{ handle },
	publish_filter{ std::make_shared<nano::network_filter> (filter_handle) }
{
}

nano::transport::tcp_channels::~tcp_channels ()
{
	rsnano::rsn_tcp_channels_destroy (handle);
}

std::size_t nano::transport::tcp_channels::size () const
{
	return rsnano::rsn_tcp_channels_channel_count (handle);
}

float nano::transport::tcp_channels::size_sqrt () const
{
	return rsnano::rsn_tcp_channels_len_sqrt (handle);
}

// Simulating with sqrt_broadcast_simulate shows we only need to broadcast to sqrt(total_peers) random peers in order to successfully publish to everyone with high probability
std::size_t nano::transport::tcp_channels::fanout (float scale) const
{
	return rsnano::rsn_tcp_channels_fanout (handle, scale);
}

std::deque<std::shared_ptr<nano::transport::channel>> nano::transport::tcp_channels::list (std::size_t count_a, uint8_t minimum_version_a)
{
	auto list_handle = rsnano::rsn_tcp_channels_random_channels (handle, count_a, minimum_version_a);
	auto vec = into_channel_vector (list_handle);
	std::deque<std::shared_ptr<nano::transport::channel>> result;
	std::move (std::begin (vec), std::end (vec), std::back_inserter (result));
	return result;
}

std::deque<std::shared_ptr<nano::transport::channel>> nano::transport::tcp_channels::random_fanout (float scale)
{
	auto list_handle = rsnano::rsn_tcp_channels_random_fanout (handle, scale);
	auto vec = into_channel_vector (list_handle);
	std::deque<std::shared_ptr<nano::transport::channel>> result;
	std::move (std::begin (vec), std::end (vec), std::back_inserter (result));
	return result;
}

std::shared_ptr<nano::transport::channel_tcp> nano::transport::tcp_channels::find_channel (nano::tcp_endpoint const & endpoint_a) const
{
	std::shared_ptr<nano::transport::channel_tcp> result;
	auto endpoint_dto{ rsnano::endpoint_to_dto (endpoint_a) };
	auto channel_handle = rsnano::rsn_tcp_channels_find_channel (handle, &endpoint_dto);
	if (channel_handle)
	{
		result = std::make_shared<nano::transport::channel_tcp> (channel_handle);
	}
	return result;
}

std::vector<std::shared_ptr<nano::transport::channel>> nano::transport::tcp_channels::random_channels (std::size_t count_a, uint8_t min_version) const
{
	auto list_handle = rsnano::rsn_tcp_channels_random_channels (handle, count_a, min_version);
	return into_channel_vector (list_handle);
}

void nano::transport::tcp_channels::random_fill (std::array<nano::endpoint, 8> & target_a) const
{
	std::array<rsnano::EndpointDto, 8> dtos;
	rsnano::rsn_tcp_channels_random_fill (handle, dtos.data ());
	auto j{ target_a.begin () };
	for (auto i{ dtos.begin () }, n{ dtos.end () }; i != n; ++i, ++j)
	{
		*j = rsnano::dto_to_udp_endpoint (*i);
	}
}

uint16_t nano::transport::tcp_channels::port () const
{
	return rsnano::rsn_tcp_channels_port (handle);
}

std::size_t nano::transport::tcp_channels::get_next_channel_id ()
{
	return rsnano::rsn_tcp_channels_get_next_channel_id (handle);
}

std::shared_ptr<nano::transport::channel_tcp> nano::transport::tcp_channels::find_node_id (nano::account const & node_id_a)
{
	std::shared_ptr<nano::transport::channel_tcp> result;
	auto channel_handle = rsnano::rsn_tcp_channels_find_node_id (handle, node_id_a.bytes.data ());
	if (channel_handle)
	{
		result = std::make_shared<nano::transport::channel_tcp> (channel_handle);
	}
	return result;
}

bool nano::transport::tcp_channels::not_a_peer (nano::endpoint const & endpoint_a, bool allow_local_peers)
{
	auto endpoint_dto{ rsnano::udp_endpoint_to_dto (endpoint_a) };
	return rsnano::rsn_tcp_channels_not_a_peer (handle, &endpoint_dto, allow_local_peers);
}

void nano::transport::tcp_channels::purge (std::chrono::system_clock::time_point const & cutoff_a)
{
	uint64_t cutoff_ns = std::chrono::duration_cast<std::chrono::nanoseconds> (cutoff_a.time_since_epoch ()).count ();
	rsnano::rsn_tcp_channels_purge (handle, cutoff_ns);
}

std::shared_ptr<nano::transport::channel> nano::transport::channel_handle_to_channel (rsnano::ChannelHandle * handle)
{
	return make_shared<nano::transport::channel_tcp> (handle);
}
