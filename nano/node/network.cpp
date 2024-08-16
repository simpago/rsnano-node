#include "nano/lib/rsnano.hpp"

#include <nano/crypto_lib/random_pool_shuffle.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/threading.hpp>
#include <nano/node/network.hpp>
#include <nano/node/node.hpp>
#include <nano/node/telemetry.hpp>

#include <boost/format.hpp>

using namespace std::chrono_literals;

/*
 * network
 */

nano::network::network (nano::node & node, uint16_t port, rsnano::SynCookiesHandle * syn_cookies_handle, rsnano::TcpChannelsHandle * channels_handle, rsnano::NetworkFilterHandle * filter_handle) :
	node{ node },
	syn_cookies{ make_shared<nano::syn_cookies> (syn_cookies_handle) },
	tcp_channels{ make_shared<nano::transport::tcp_channels> (channels_handle, filter_handle) }
{
}

nano::network::~network ()
{
}

namespace
{
void callback_wrapper (void * context)
{
	if (context == nullptr)
		return;

	auto callback = static_cast<std::function<void ()> *> (context);
	(*callback) ();
}

void drop_context (void * context)
{
	if (context == nullptr)
		return;

	auto callback = static_cast<std::function<void ()> *> (context);
	delete callback;
}
}

void nano::network::flood_block_many (std::deque<std::shared_ptr<nano::block>> blocks_a, std::function<void ()> callback_a, unsigned delay_a)
{
	rsnano::block_vec block_vec{ blocks_a };
	auto context = callback_a != nullptr ? new std::function<void ()> (callback_a) : nullptr;
	rsnano::rsn_node_flood_block_many (node.handle, block_vec.handle, delay_a, callback_wrapper, context, drop_context);
}

void nano::network::inbound (const nano::message & message, const std::shared_ptr<nano::transport::channel> & channel)
{
	rsnano::rsn_node_inbound (node.handle, message.handle, channel->handle);
}

// Send keepalives to all the peers we've been notified of
void nano::network::merge_peers (std::array<nano::endpoint, 8> const & peers_a)
{
	for (auto i (peers_a.begin ()), j (peers_a.end ()); i != j; ++i)
	{
		merge_peer (*i);
	}
}

void nano::network::merge_peer (nano::endpoint const & peer_a)
{
	auto peer_dto{ rsnano::udp_endpoint_to_dto (peer_a) };
	rsnano::rsn_node_connect (node.handle, &peer_dto);
}

std::vector<std::shared_ptr<nano::transport::channel>> nano::network::random_channels (std::size_t count_a, uint8_t min_version_a) const
{
	return tcp_channels->random_channels (count_a, min_version_a);
}

std::shared_ptr<nano::transport::channel> nano::network::find_node_id (nano::account const & node_id_a)
{
	return tcp_channels->find_node_id (node_id_a);
}

nano::endpoint nano::network::endpoint () const
{
	return nano::endpoint (boost::asio::ip::address_v6::loopback (), tcp_channels->port ());
}

void nano::network::cleanup (std::chrono::system_clock::time_point const & cutoff_a)
{
	tcp_channels->purge (cutoff_a);
}

std::size_t nano::network::size () const
{
	return tcp_channels->size ();
}

bool nano::network::empty () const
{
	return size () == 0;
}

/*
 * syn_cookies
 */

nano::syn_cookies::syn_cookies (rsnano::SynCookiesHandle * handle) :
	handle{ handle }
{
}

nano::syn_cookies::~syn_cookies ()
{
	rsnano::rsn_syn_cookies_destroy (handle);
}

std::size_t nano::syn_cookies::cookies_size ()
{
	return rsnano::rsn_syn_cookies_cookies_count (handle);
}

std::string nano::network::to_string (nano::networks network)
{
	rsnano::StringDto result;
	rsnano::rsn_network_to_string (static_cast<uint16_t> (network), &result);
	return rsnano::convert_dto_to_string (result);
}
