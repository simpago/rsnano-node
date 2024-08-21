#pragma once

#include "nano/lib/rsnano.hpp"

#include <nano/node/common.hpp>
#include <nano/node/transport/tcp.hpp>
#include <nano/secure/network_filter.hpp>

#include <boost/thread/thread.hpp>

#include <chrono>
#include <deque>
#include <memory>

namespace nano
{
class node;

class network final : public std::enable_shared_from_this<network>
{
public:
	network (nano::node &, uint16_t port, rsnano::TcpChannelsHandle * channels_handle, rsnano::NetworkFilterHandle * filter_handle);
	~network ();

	// Flood block to a random selection of peers
	void flood_block_many (std::deque<std::shared_ptr<nano::block>>, std::function<void ()> = nullptr, unsigned = broadcast_interval_ms);
	void merge_peers (std::array<nano::endpoint, 8> const &);
	void merge_peer (nano::endpoint const &);
	nano::endpoint endpoint () const;
	std::size_t size () const;
	bool empty () const;

	static std::string to_string (nano::networks);

private: // Dependencies
	nano::node & node;

public:
	std::shared_ptr<nano::transport::tcp_channels> tcp_channels;
	std::atomic<uint16_t> port{ 0 };

public:
	static unsigned const broadcast_interval_ms = 10;

	static std::size_t const confirm_req_hashes_max = 7;
};
}
