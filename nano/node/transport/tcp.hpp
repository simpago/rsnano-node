#pragma once

#include "nano/node/nodeconfig.hpp"

#include <nano/node/common.hpp>
#include <nano/node/transport/transport.hpp>

#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/mem_fun.hpp>
#include <boost/multi_index/member.hpp>
#include <boost/multi_index/ordered_index.hpp>
#include <boost/multi_index/random_access_index.hpp>
#include <boost/multi_index_container.hpp>

#include <unordered_set>

namespace mi = boost::multi_index;

namespace nano
{
class bootstrap_server;
class node_config;
class node_flags;
class network;
class telemetry;
class request_response_visitor_factory;
class syn_cookies;
class tcp_message_item final
{
public:
	tcp_message_item ();
	explicit tcp_message_item (rsnano::TcpMessageItemHandle * handle_a);
	tcp_message_item (std::shared_ptr<nano::message> message_a, nano::tcp_endpoint endpoint_a, nano::account node_id_a, std::shared_ptr<nano::socket> socket_a);
	tcp_message_item (nano::tcp_message_item const & other_a);
	tcp_message_item (nano::tcp_message_item && other_a) noexcept;
	~tcp_message_item ();
	tcp_message_item & operator= (tcp_message_item const & other_a);
	tcp_message_item & operator= (tcp_message_item && other_a);
	std::shared_ptr<nano::message> get_message () const;
	nano::tcp_endpoint get_endpoint () const;
	nano::account get_node_id () const;
	std::shared_ptr<nano::socket> get_socket () const;
	rsnano::TcpMessageItemHandle * handle;
};
namespace transport
{
	class tcp_server;
	class tcp_channels;

	class channel_tcp_observer
	{
	public:
		virtual void data_sent (boost::asio::ip::tcp::endpoint const & endpoint_a) = 0;
		virtual void host_unreachable () = 0;
		virtual void message_sent (nano::message const & message_a) = 0;
		virtual void message_dropped (nano::message const & message_a, std::size_t buffer_size_a) = 0;
		virtual void no_socket_drop () = 0;
		virtual void write_drop () = 0;
	};

	void channel_tcp_send_callback (void * context_a, const rsnano::ErrorCodeDto * ec_a, std::size_t size_a);
	void delete_send_buffer_callback (void * context_a);

	class channel_tcp : public nano::transport::channel
	{
		friend class nano::transport::tcp_channels;

	public:
		channel_tcp (boost::asio::io_context & io_ctx_a, nano::outbound_bandwidth_limiter & limiter_a, nano::network_constants const & network_a, std::shared_ptr<nano::socket> const & socket_a, std::shared_ptr<nano::transport::channel_tcp_observer> const & observer_a);
		channel_tcp (rsnano::ChannelHandle * handle_a) :
			channel{ handle_a } {};

		uint8_t get_network_version () const override;
		void set_network_version (uint8_t network_version_a) override;
		std::size_t hash_code () const override;
		bool operator== (nano::transport::channel const &) const override;
		void send (nano::message & message_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a = nullptr, nano::buffer_drop_policy policy_a = nano::buffer_drop_policy::limiter, nano::bandwidth_limit_type = nano::bandwidth_limit_type::standard) override;
		// TODO: investigate clang-tidy warning about default parameters on virtual/override functions
		//
		void send_buffer (nano::shared_const_buffer const &, std::function<void (boost::system::error_code const &, std::size_t)> const & = nullptr, nano::buffer_drop_policy = nano::buffer_drop_policy::limiter) override;
		std::string to_string () const override;
		bool operator== (nano::transport::channel_tcp const & other_a) const;
		std::shared_ptr<nano::socket> try_get_socket () const;

		void set_endpoint ();

		nano::endpoint get_endpoint () const override
		{
			return nano::transport::map_tcp_to_endpoint (get_tcp_endpoint ());
		}

		nano::tcp_endpoint get_tcp_endpoint () const override;
		nano::transport::transport_type get_type () const override
		{
			return nano::transport::transport_type::tcp;
		}

		bool max () override;
		nano::endpoint get_peering_endpoint () const override;
		void set_peering_endpoint (nano::endpoint endpoint) override;
	};

	class tcp_server_factory
	{
	public:
		tcp_server_factory (nano::node & node_a);
		std::shared_ptr<nano::transport::tcp_server> create_tcp_server (const std::shared_ptr<nano::transport::channel_tcp> & channel_a, const std::shared_ptr<nano::socket> & socket_a);

	private:
		nano::node & node;
	};

	class tcp_channels final : public nano::transport::channel_tcp_observer, public std::enable_shared_from_this<tcp_channels>
	{
		friend class nano::transport::channel_tcp;

	public:
		explicit tcp_channels (nano::node &, std::function<void (nano::message const &, std::shared_ptr<nano::transport::channel> const &)> = nullptr);
		tcp_channels (nano::transport::tcp_channels const &) = delete;
		~tcp_channels ();
		bool insert (std::shared_ptr<nano::transport::channel_tcp> const &, std::shared_ptr<nano::socket> const &, std::shared_ptr<nano::transport::tcp_server> const &);
		void erase (nano::tcp_endpoint const &);
		void erase_temporary_channel (nano::tcp_endpoint const &);
		std::size_t size () const;
		std::shared_ptr<nano::transport::channel_tcp> find_channel (nano::tcp_endpoint const &) const;
		std::unordered_set<std::shared_ptr<nano::transport::channel>> random_set (std::size_t, uint8_t = 0, bool = false) const;
		std::vector<endpoint> get_current_peers () const;
		std::shared_ptr<nano::transport::channel_tcp> find_node_id (nano::account const &);
		// Get the next peer for attempting a tcp connection
		nano::tcp_endpoint bootstrap_peer (uint8_t connection_protocol_version_min);
		void receive ();
		void start ();
		void stop ();
		void process_messages ();
		void process_message (nano::message const &, nano::tcp_endpoint const &, nano::account const &, std::shared_ptr<nano::socket> const &);
		bool max_ip_connections (nano::tcp_endpoint const & endpoint_a);
		bool max_subnetwork_connections (nano::tcp_endpoint const & endpoint_a);
		bool max_ip_or_subnetwork_connections (nano::tcp_endpoint const & endpoint_a);
		// Should we reach out to this endpoint with a keepalive message
		bool reachout (nano::endpoint const &);
		std::unique_ptr<container_info_component> collect_container_info (std::string const &);
		void purge (std::chrono::steady_clock::time_point const &);
		void ongoing_keepalive ();
		void list (std::deque<std::shared_ptr<nano::transport::channel>> &, uint8_t = 0, bool = true);
		void modify (std::shared_ptr<nano::transport::channel_tcp> const &, std::function<void (std::shared_ptr<nano::transport::channel_tcp> const &)>);
		void update (nano::tcp_endpoint const &);
		// Connection start
		void start_tcp (nano::endpoint const &);
		void start_tcp_receive_node_id (std::shared_ptr<nano::transport::channel_tcp> const &, nano::endpoint const &, std::shared_ptr<std::vector<uint8_t>> const &);
		void on_new_channel (std::function<void (std::shared_ptr<nano::transport::channel>)> observer_a);

		// channel_tcp_observer:
		void data_sent (boost::asio::ip::tcp::endpoint const & endpoint_a) override;
		void host_unreachable () override;
		void message_sent (nano::message const & message_a) override;
		void message_dropped (nano::message const & message_a, std::size_t buffer_size_a) override;
		void no_socket_drop () override;
		void write_drop () override;

	private:
		std::function<void (nano::message const &, std::shared_ptr<nano::transport::channel> const &)> sink;
		class endpoint_tag
		{
		};
		class ip_address_tag
		{
		};
		class subnetwork_tag
		{
		};
		class random_access_tag
		{
		};
		class last_packet_sent_tag
		{
		};
		class last_bootstrap_attempt_tag
		{
		};
		class last_attempt_tag
		{
		};
		class node_id_tag
		{
		};
		class version_tag
		{
		};

		class channel_tcp_wrapper final
		{
		public:
			channel_tcp_wrapper (std::shared_ptr<nano::transport::channel_tcp> channel_a, std::shared_ptr<nano::socket> socket_a, std::shared_ptr<nano::transport::tcp_server> server_a);
			channel_tcp_wrapper (channel_tcp_wrapper const &) = delete;
			~channel_tcp_wrapper ();
			std::shared_ptr<nano::transport::channel_tcp> get_channel () const;
			std::shared_ptr<nano::transport::tcp_server> get_response_server () const;
			nano::tcp_endpoint endpoint () const
			{
				return get_channel ()->get_tcp_endpoint ();
			}
			std::chrono::steady_clock::time_point last_packet_sent () const
			{
				return get_channel ()->get_last_packet_sent ();
			}
			std::chrono::steady_clock::time_point last_bootstrap_attempt () const
			{
				return get_channel ()->get_last_bootstrap_attempt ();
			}
			std::shared_ptr<nano::socket> try_get_socket () const
			{
				return get_channel ()->try_get_socket ();
			}
			boost::asio::ip::address ip_address () const
			{
				return nano::transport::ipv4_address_or_ipv6_subnet (endpoint ().address ());
			}
			boost::asio::ip::address subnetwork () const
			{
				return nano::transport::map_address_to_subnetwork (endpoint ().address ());
			}
			nano::account node_id () const
			{
				auto node_id (get_channel ()->get_node_id ());
				return node_id;
			}
			uint8_t network_version () const
			{
				return get_channel ()->get_network_version ();
			}

		private:
			// Keep shared_ptrs in C++ so that we don't have to return new shared_ptrs.
			// New shared_ptrs would break unordered_maps of channels!
			std::shared_ptr<nano::transport::channel_tcp> channel;
			std::shared_ptr<nano::transport::tcp_server> server;
			rsnano::ChannelTcpWrapperHandle * handle;
		};
		class tcp_endpoint_attempt final
		{
		public:
			nano::tcp_endpoint endpoint;
			boost::asio::ip::address address;
			boost::asio::ip::address subnetwork;
			std::chrono::steady_clock::time_point last_attempt{ std::chrono::steady_clock::now () };

			explicit tcp_endpoint_attempt (nano::tcp_endpoint const & endpoint_a) :
				endpoint (endpoint_a),
				address (nano::transport::ipv4_address_or_ipv6_subnet (endpoint_a.address ())),
				subnetwork (nano::transport::map_address_to_subnetwork (endpoint_a.address ()))
			{
			}
		};
		nano::transport::tcp_server_factory tcp_server_factory;
		nano::keypair node_id;
		nano::network_params & network_params;
		nano::outbound_bandwidth_limiter & limiter;
		std::shared_ptr<nano::syn_cookies> syn_cookies;
		std::shared_ptr<nano::stat> stats;
		std::shared_ptr<nano::node_config> config;
		std::shared_ptr<nano::logger_mt> logger;
		std::shared_ptr<nano::network> network;
		std::shared_ptr<nano::thread_pool> workers;
		nano::node_flags flags;
		boost::asio::io_context & io_ctx;
		mutable nano::mutex mutex;

	public:
		// clang-format off
		boost::multi_index_container<channel_tcp_wrapper,
		mi::indexed_by<
			mi::random_access<mi::tag<random_access_tag>>,
			mi::ordered_non_unique<mi::tag<last_bootstrap_attempt_tag>,
				mi::const_mem_fun<channel_tcp_wrapper, std::chrono::steady_clock::time_point, &channel_tcp_wrapper::last_bootstrap_attempt>>,
			mi::hashed_unique<mi::tag<endpoint_tag>,
				mi::const_mem_fun<channel_tcp_wrapper, nano::tcp_endpoint, &channel_tcp_wrapper::endpoint>>,
			mi::hashed_non_unique<mi::tag<node_id_tag>,
				mi::const_mem_fun<channel_tcp_wrapper, nano::account, &channel_tcp_wrapper::node_id>>,
			mi::ordered_non_unique<mi::tag<last_packet_sent_tag>,
				mi::const_mem_fun<channel_tcp_wrapper, std::chrono::steady_clock::time_point, &channel_tcp_wrapper::last_packet_sent>>,
			mi::ordered_non_unique<mi::tag<version_tag>,
				mi::const_mem_fun<channel_tcp_wrapper, uint8_t, &channel_tcp_wrapper::network_version>>,
			mi::hashed_non_unique<mi::tag<ip_address_tag>,
				mi::const_mem_fun<channel_tcp_wrapper, boost::asio::ip::address, &channel_tcp_wrapper::ip_address>>,
			mi::hashed_non_unique<mi::tag<subnetwork_tag>,
				mi::const_mem_fun<channel_tcp_wrapper, boost::asio::ip::address, &channel_tcp_wrapper::subnetwork>>>>
		channels;
private:
		boost::multi_index_container<tcp_endpoint_attempt,
		mi::indexed_by<
			mi::hashed_unique<mi::tag<endpoint_tag>,
				mi::member<tcp_endpoint_attempt, nano::tcp_endpoint, &tcp_endpoint_attempt::endpoint>>,
			mi::hashed_non_unique<mi::tag<ip_address_tag>,
				mi::member<tcp_endpoint_attempt, boost::asio::ip::address, &tcp_endpoint_attempt::address>>,
			mi::hashed_non_unique<mi::tag<subnetwork_tag>,
				mi::member<tcp_endpoint_attempt, boost::asio::ip::address, &tcp_endpoint_attempt::subnetwork>>,
			mi::ordered_non_unique<mi::tag<last_attempt_tag>,
				mi::member<tcp_endpoint_attempt, std::chrono::steady_clock::time_point, &tcp_endpoint_attempt::last_attempt>>>>
		attempts;
		// clang-format on
		std::atomic<bool> stopped{ false };
		// Called when a new channel is observed
		std::function<void (std::shared_ptr<nano::transport::channel>)> channel_observer;
		rsnano::TcpChannelsHandle * handle;

		friend class network_peer_max_tcp_attempts_subnetwork_Test;
	};
} // namespace transport
} // namespace nano
