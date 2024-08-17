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

/**
 * Node ID cookies for node ID handshakes
 */
class syn_cookies final
{
public:
	explicit syn_cookies (rsnano::SynCookiesHandle * handle);
	syn_cookies (nano::syn_cookies const &) = delete;
	~syn_cookies ();

	std::size_t cookies_size ();
	rsnano::SynCookiesHandle * handle;
};

class network final : public std::enable_shared_from_this<network>
{
public:
	network (nano::node &, uint16_t port, rsnano::SynCookiesHandle * syn_cookies_handle, rsnano::TcpChannelsHandle * channels_handle, rsnano::NetworkFilterHandle * filter_handle);
	~network ();

	// Flood block to a random selection of peers
	void flood_block_many (std::deque<std::shared_ptr<nano::block>>, std::function<void ()> = nullptr, unsigned = broadcast_interval_ms);
	void merge_peers (std::array<nano::endpoint, 8> const &);
	void merge_peer (nano::endpoint const &);
	nano::endpoint endpoint () const;
	void cleanup (std::chrono::system_clock::time_point const & cutoff);
	std::size_t size () const;
	bool empty () const;
	/** Disconnects and adds peer to exclusion list */
	void inbound (nano::message const &, std::shared_ptr<nano::transport::channel> const &);

	static std::string to_string (nano::networks);

private: // Dependencies
	nano::node & node;

public:
	std::shared_ptr<nano::syn_cookies> syn_cookies;
	std::shared_ptr<nano::transport::tcp_channels> tcp_channels;
	std::atomic<uint16_t> port{ 0 };

public:
	static unsigned const broadcast_interval_ms = 10;

	static std::size_t const confirm_req_hashes_max = 7;
};
}
