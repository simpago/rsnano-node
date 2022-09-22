#pragma once

#include <nano/node/bootstrap/bootstrap_bulk_pull.hpp>
#include <nano/node/common.hpp>
#include <nano/node/socket.hpp>

#include <atomic>

namespace nano
{
class node;
namespace transport
{
	class channel_tcp;
}

class bootstrap_attempt;
class bootstrap_connections;
class frontier_req_client;
class pull_info;

class bootstrap_client_observer
{
public:
	virtual void bootstrap_client_closed () = 0;
};

/**
 * Owns the client side of the bootstrap connection.
 */
class bootstrap_client final : public std::enable_shared_from_this<bootstrap_client>
{
public:
	bootstrap_client (std::shared_ptr<nano::bootstrap_client_observer> const & observer_a, std::shared_ptr<nano::transport::channel_tcp> const & channel_a, std::shared_ptr<nano::socket> const & socket_a);
	~bootstrap_client ();
	void stop (bool force);
	double sample_block_rate ();
	double elapsed_seconds () const;
	void set_start_time ();
	void async_read (std::size_t size_a, std::function<void (boost::system::error_code const &, std::size_t)> callback_a);
	void close_socket ();
	void set_timeout (std::chrono::seconds timeout_a);
	uint8_t * get_receive_buffer ();
	nano::tcp_endpoint remote_endpoint () const;
	std::string channel_string () const;
	void send (nano::message & message_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a = nullptr, nano::buffer_drop_policy drop_policy_a = nano::buffer_drop_policy::limiter);
	void send_buffer (nano::shared_const_buffer const & buffer_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a = nullptr, nano::buffer_drop_policy policy_a = nano::buffer_drop_policy::limiter);
	nano::tcp_endpoint get_tcp_endpoint () const;
	std::shared_ptr<nano::socket> get_socket () const;
	uint64_t get_block_count () const;
	uint64_t inc_block_count (); // returns the previous block count
	double get_block_rate () const;
	bool get_pending_stop () const;
	bool get_hard_stop () const;

private:
	std::vector<uint8_t> buffer; // only used for returning a uint8_t*
	rsnano::BootstrapClientHandle * handle;
};

/**
 * Container for bootstrap_client objects. Owned by bootstrap_initiator which pools open connections and makes them available
 * for use by different bootstrap sessions.
 */
class bootstrap_connections final : public std::enable_shared_from_this<bootstrap_connections>, public bootstrap_client_observer
{
public:
	explicit bootstrap_connections (nano::node & node_a);
	std::shared_ptr<nano::bootstrap_client> connection (std::shared_ptr<nano::bootstrap_attempt> const & attempt_a = nullptr, bool use_front_connection = false);
	void pool_connection (std::shared_ptr<nano::bootstrap_client> const & client_a, bool new_client = false, bool push_front = false);
	void add_connection (nano::endpoint const & endpoint_a);
	std::shared_ptr<nano::bootstrap_client> find_connection (nano::tcp_endpoint const & endpoint_a);
	void connect_client (nano::tcp_endpoint const & endpoint_a, bool push_front = false);
	unsigned target_connections (std::size_t pulls_remaining, std::size_t attempts_count) const;
	void populate_connections (bool repeat = true);
	void start_populate_connections ();
	void add_pull (nano::pull_info const & pull_a);
	void request_pull (nano::unique_lock<nano::mutex> & lock_a);
	void requeue_pull (nano::pull_info const & pull_a, bool network_error = false);
	void clear_pulls (uint64_t);
	void run ();
	void stop ();
	void bootstrap_client_closed () override;
	std::deque<std::weak_ptr<nano::bootstrap_client>> clients;
	std::atomic<unsigned> connections_count{ 0 };
	nano::node & node;
	std::deque<std::shared_ptr<nano::bootstrap_client>> idle;
	std::deque<nano::pull_info> pulls;
	std::atomic<bool> populate_connections_started{ false };
	std::atomic<bool> new_connections_empty{ false };
	std::atomic<bool> stopped{ false };
	nano::mutex mutex;
	nano::condition_variable condition;
};
}
