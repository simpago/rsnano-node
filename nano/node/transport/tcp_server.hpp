#pragma once

#include <nano/node/common.hpp>
#include <nano/node/messages.hpp>
#include <nano/node/socket.hpp>

#include <atomic>

namespace rsnano
{
class BootstrapServerHandle;
class BootstrapServerWeakHandle;
}

namespace nano
{
class tcp_server;
class node_config;
class node_flags;
class network;

namespace transport
{
	class tcp_channels;
}

class tcp_server_observer
{
public:
	virtual void tcp_server_timeout (std::uintptr_t inner_ptr) = 0;
	virtual void tcp_server_exited (nano::socket::type_t type_a, std::uintptr_t inner_ptr, nano::tcp_endpoint const &) = 0;
	virtual std::size_t get_bootstrap_count () = 0;
	virtual void inc_bootstrap_count () = 0;
	virtual void inc_realtime_count () = 0;
};

class tcp_server_weak_wrapper
{
public:
	tcp_server_weak_wrapper () = default;
	explicit tcp_server_weak_wrapper (std::shared_ptr<nano::transport::tcp_server> const & server);
	tcp_server_weak_wrapper (tcp_server_weak_wrapper const &);
	tcp_server_weak_wrapper (tcp_server_weak_wrapper &&) noexcept;
	~tcp_server_weak_wrapper ();
	tcp_server_weak_wrapper & operator= (tcp_server_weak_wrapper && other_a) noexcept;
	[[nodiscard]] std::shared_ptr<nano::transport::tcp_server> lock () const;

private:
	rsnano::BootstrapServerWeakHandle * handle{ nullptr };
};
class message;
class tcp_message_manager;
class syn_cookies;
}

namespace nano::transport
{
class message_deserializer;
class tcp_server;

/**
 * Server side portion of bootstrap sessions. Listens for new socket connections and spawns tcp_server objects when connected.
 */
class tcp_listener final : public nano::tcp_server_observer
{
public:
	tcp_listener (uint16_t, nano::node &);
	void start ();
	void stop ();
	void accept_action (boost::system::error_code const &, std::shared_ptr<nano::socket> const &);
	std::size_t connection_count ();
	void erase_connection (std::uintptr_t conn_ptr);

	std::size_t get_bootstrap_count () override;
	void inc_bootstrap_count () override;
	void dec_bootstrap_count ();

	std::size_t get_realtime_count ();
	void inc_realtime_count () override;
	void dec_realtime_count ();

	void tcp_server_timeout (std::uintptr_t inner_ptr) override;
	void tcp_server_exited (nano::socket::type_t type_a, std::uintptr_t inner_ptr_a, nano::tcp_endpoint const & endpoint_a) override;

	nano::mutex mutex;
	std::unordered_map<std::size_t, tcp_server_weak_wrapper> connections;
	nano::tcp_endpoint endpoint ();
	std::shared_ptr<nano::node_config> config;
	std::shared_ptr<nano::logger_mt> logger;
	std::shared_ptr<nano::network> network;
	nano::node & node;
	std::shared_ptr<nano::server_socket> listening_socket;
	bool on{ false };
	uint16_t port;

private:
	std::atomic<std::size_t> bootstrap_count{ 0 };
	std::atomic<std::size_t> realtime_count{ 0 };
};

std::unique_ptr<container_info_component> collect_container_info (tcp_listener & bootstrap_listener, std::string const & name);

class request_response_visitor_factory
{
public:
	explicit request_response_visitor_factory (nano::node & node_a);
	std::shared_ptr<nano::message_visitor> create_bootstrap (std::shared_ptr<nano::transport::tcp_server> connection_a);

private:
	nano::node & node; // shared_ptr isn't possible, because this factory gets created in node's constructor
};

namespace bootstrap
{
	class message_deserializer;
};

class tcp_server final : public std::enable_shared_from_this<nano::transport::tcp_server>
{
public:
	tcp_server (
	boost::asio::io_context & io_ctx_a,
	std::shared_ptr<nano::socket> const & socket_a,
	std::shared_ptr<nano::logger_mt> const & logger_a,
	nano::stat const & stats_a,
	nano::node_flags const & flags_a,
	nano::node_config const & config_a,
	std::shared_ptr<nano::tcp_server_observer> const & observer_a,
	std::shared_ptr<nano::transport::request_response_visitor_factory> visitor_factory_a,
	std::shared_ptr<nano::thread_pool> const & workers_a,
	nano::network_filter const & publish_filter_a,
	nano::block_uniquer & block_uniquer_a,
	nano::vote_uniquer & vote_uniquer_a,
	nano::tcp_message_manager & tcp_message_manager_a,
	nano::syn_cookies & syn_cookies_a,
	nano::keypair & node_id_a,
	bool allow_bootstrap_a = true);
	explicit tcp_server (rsnano::TcpServerHandle * handle_a);
	tcp_server (nano::transport::tcp_server const &) = delete;
	tcp_server (nano::transport::tcp_server &&) = delete;
	~tcp_server ();
	void start ();
	void stop ();
	void timeout ();
	void send_handshake_response (nano::uint256_union query);
	bool is_stopped () const;
	std::size_t unique_id () const;
	void set_remote_node_id (nano::account account_a);
	nano::tcp_endpoint get_remote_endpoint () const;
	std::shared_ptr<nano::socket> const get_socket () const;

	rsnano::TcpServerHandle * handle;

	class bootstrap_message_visitor : public nano::message_visitor
	{
	public:
		bool processed{ false };

		explicit bootstrap_message_visitor (std::shared_ptr<tcp_server>, std::shared_ptr<nano::node>);

		void bulk_pull (nano::bulk_pull const &) override;
		void bulk_pull_account (nano::bulk_pull_account const &) override;
		void bulk_push (nano::bulk_push const &) override;
		void frontier_req (nano::frontier_req const &) override;

	private:
		std::shared_ptr<tcp_server> server;
		std::shared_ptr<nano::node> node;
	};
};
}
