#include <nano/lib/rsnanoutils.hpp>
#include <nano/node/bootstrap/bootstrap_bulk_push.hpp>
#include <nano/node/bootstrap/bootstrap_frontier.hpp>
#include <nano/node/messages.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/message_deserializer.hpp>
#include <nano/node/transport/tcp.hpp>
#include <nano/node/transport/tcp_server.hpp>

#include <boost/format.hpp>

nano::tcp_server_weak_wrapper::tcp_server_weak_wrapper (std::shared_ptr<nano::transport::tcp_server> const & server) :
	handle{ rsnano::rsn_bootstrap_server_get_weak (server->handle) }
{
}

nano::tcp_server_weak_wrapper::tcp_server_weak_wrapper (tcp_server_weak_wrapper const & other_a) :
	handle{ rsnano::rsn_bootstrap_server_copy_weak (other_a.handle) }
{
}

nano::tcp_server_weak_wrapper::tcp_server_weak_wrapper (tcp_server_weak_wrapper && other_a) noexcept :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::tcp_server_weak_wrapper::~tcp_server_weak_wrapper ()
{
	if (handle)
		rsnano::rsn_bootstrap_server_destroy_weak (handle);
}

nano::tcp_server_weak_wrapper & nano::tcp_server_weak_wrapper::operator= (tcp_server_weak_wrapper && other_a) noexcept
{
	if (handle)
		rsnano::rsn_bootstrap_server_destroy_weak (handle);
	handle = other_a.handle;
	other_a.handle = nullptr;
	return *this;
}

std::shared_ptr<nano::transport::tcp_server> nano::tcp_server_weak_wrapper::lock () const
{
	auto server_handle = rsnano::rsn_bootstrap_server_lock_weak (handle);
	if (server_handle)
		return std::make_shared<nano::transport::tcp_server> (server_handle);

	return {};
}

nano::transport::tcp_listener::tcp_listener (uint16_t port_a, nano::node & node_a) :
	config{ node_a.config },
	logger{ node_a.logger },
	network{ node_a.network },
	node (node_a),
	port (port_a)
{
}

void nano::transport::tcp_listener::start ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	on = true;
	listening_socket = std::make_shared<nano::server_socket> (node, boost::asio::ip::tcp::endpoint (boost::asio::ip::address_v6::any (), port), config->tcp_incoming_connections_max);
	boost::system::error_code ec;
	listening_socket->start (ec);
	if (ec)
	{
		logger->always_log (boost::str (boost::format ("Network: Error while binding for incoming TCP/bootstrap on port %1%: %2%") % listening_socket->listening_port () % ec.message ()));
		throw std::runtime_error (ec.message ());
	}

	// the user can either specify a port value in the config or it can leave the choice up to the OS;
	// independently of user's port choice, he may have also opted to disable UDP or not; this gives us 4 possibilities:
	// (1): UDP enabled, port specified
	// (2): UDP enabled, port not specified
	// (3): UDP disabled, port specified
	// (4): UDP disabled, port not specified
	//
	const auto listening_port = listening_socket->listening_port ();
	if (!node.flags.disable_udp ())
	{
		// (1) and (2) -- no matter if (1) or (2), since UDP socket binding happens before this TCP socket binding,
		// we must have already been constructed with a valid port value, so check that it really is the same everywhere
		//
		debug_assert (port == listening_port);
		debug_assert (port == network->port);
		debug_assert (port == network->endpoint ().port ());
	}
	else
	{
		// (3) -- nothing to do, just check that port values match everywhere
		//
		if (port == listening_port)
		{
			debug_assert (port == network->port);
			debug_assert (port == network->endpoint ().port ());
		}
		// (4) -- OS port choice happened at TCP socket bind time, so propagate this port value back;
		// the propagation is done here for the `tcp_listener` itself, whereas for `network`, the node does it
		// after calling `tcp_listener.start ()`
		//
		else
		{
			port = listening_port;
		}
	}

	listening_socket->on_connection ([this] (std::shared_ptr<nano::socket> const & new_connection, boost::system::error_code const & ec_a) {
		if (!ec_a)
		{
			accept_action (ec_a, new_connection);
		}
		return true;
	});
}

void nano::transport::tcp_listener::stop ()
{
	decltype (connections) connections_l;
	{
		nano::lock_guard<nano::mutex> lock{ mutex };
		on = false;
		connections_l.swap (connections);
	}
	if (listening_socket)
	{
		nano::lock_guard<nano::mutex> lock{ mutex };
		listening_socket->close ();
		listening_socket = nullptr;
	}
}

std::size_t nano::transport::tcp_listener::connection_count ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	return connections.size ();
}

void nano::transport::tcp_listener::erase_connection (std::uintptr_t conn_ptr)
{
	nano::lock_guard<nano::mutex> lock (mutex);
	connections.erase (conn_ptr);
}

std::size_t nano::transport::tcp_listener::get_bootstrap_count ()
{
	return bootstrap_count;
}

void nano::transport::tcp_listener::inc_bootstrap_count ()
{
	++bootstrap_count;
}

void nano::transport::tcp_listener::dec_bootstrap_count ()
{
	--bootstrap_count;
}

std::size_t nano::transport::tcp_listener::get_realtime_count ()
{
	return realtime_count;
}

void nano::transport::tcp_listener::inc_realtime_count ()
{
	++realtime_count;
}

void nano::transport::tcp_listener::dec_realtime_count ()
{
	--realtime_count;
}

void nano::transport::tcp_listener::tcp_server_timeout (std::uintptr_t inner_ptr)
{
	if (config->logging.bulk_pull_logging ())
	{
		logger->try_log ("Closing incoming tcp / bootstrap server by timeout");
	}
	{
		erase_connection (inner_ptr);
	}
}

void nano::transport::tcp_listener::tcp_server_exited (nano::socket::type_t type_a, std::uintptr_t inner_ptr_a, nano::tcp_endpoint const & endpoint_a)
{
	if (config->logging.bulk_pull_logging ())
	{
		logger->try_log ("Exiting incoming TCP/bootstrap server");
	}
	if (type_a == nano::socket::type_t::bootstrap)
	{
		dec_bootstrap_count ();
	}
	else if (type_a == nano::socket::type_t::realtime)
	{
		dec_realtime_count ();
		// Clear temporary channel
		network->tcp_channels->erase_temporary_channel (endpoint_a);
	}
	erase_connection (inner_ptr_a);
}

void nano::transport::tcp_listener::accept_action (boost::system::error_code const & ec, std::shared_ptr<nano::socket> const & socket_a)
{
	if (!network->excluded_peers.check (socket_a->remote_endpoint ()))
	{
		auto req_resp_visitor_factory = std::make_shared<nano::transport::request_response_visitor_factory> (node);
		auto server (std::make_shared<nano::transport::tcp_server> (
		node.io_ctx, socket_a, logger,
		*node.stats, node.flags, *config,
		node.tcp_listener, req_resp_visitor_factory, node.workers,
		*network->publish_filter,
		node.block_uniquer, node.vote_uniquer, node.network->tcp_message_manager, *network->syn_cookies, node.node_id, true));
		nano::lock_guard<nano::mutex> lock{ mutex };
		connections[server->unique_id ()] = nano::tcp_server_weak_wrapper (server);
		server->start ();
	}
	else
	{
		node.stats->inc (nano::stat::type::tcp, nano::stat::detail::tcp_excluded);
		if (config->logging.network_rejected_logging ())
		{
			logger->try_log ("Rejected connection from excluded peer ", socket_a->remote_endpoint ());
		}
	}
}

boost::asio::ip::tcp::endpoint nano::transport::tcp_listener::endpoint ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	if (on && listening_socket)
	{
		return { boost::asio::ip::address_v6::loopback (), port };
	}
	else
	{
		return { boost::asio::ip::address_v6::loopback (), 0 };
	}
}

std::unique_ptr<nano::container_info_component> nano::transport::collect_container_info (nano::transport::tcp_listener & bootstrap_listener, std::string const & name)
{
	//auto sizeof_element = sizeof (decltype (bootstrap_listener.connections)::value_type);
	size_t sizeof_element = 1;
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "connections", bootstrap_listener.connection_count (), sizeof_element }));
	return composite;
}

nano::transport::tcp_server::tcp_server (
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
bool allow_bootstrap_a)
{
	auto config_dto{ config_a.to_dto () };
	auto observer_handle = new std::shared_ptr<nano::tcp_server_observer> (observer_a);
	auto network_dto{ config_a.network_params.to_dto () };
	rsnano::io_ctx_wrapper io_ctx (io_ctx_a);
	rsnano::CreateTcpServerParams params;
	params.socket = socket_a->handle;
	params.config = &config_dto;
	params.logger = nano::to_logger_handle (logger_a);
	params.observer = observer_handle;
	params.publish_filter = publish_filter_a.handle;
	params.workers = new std::shared_ptr<nano::thread_pool> (workers_a);
	params.io_ctx = io_ctx.handle ();
	params.network = &network_dto.dto;
	params.disable_bootstrap_listener = flags_a.disable_bootstrap_listener ();
	params.connections_max = config_a.bootstrap_connections_max;
	params.stats = stats_a.handle;
	params.disable_bootstrap_bulk_pull_server = flags_a.disable_bootstrap_bulk_pull_server ();
	params.disable_tcp_realtime = flags_a.disable_tcp_realtime ();
	params.request_response_visitor_factory = new std::shared_ptr<nano::transport::request_response_visitor_factory> (visitor_factory_a);
	params.block_uniquer = block_uniquer_a.handle;
	params.vote_uniquer = vote_uniquer_a.handle;
	params.tcp_message_manager = tcp_message_manager_a.handle;
	params.syn_cookies = syn_cookies_a.handle;
	params.node_id_prv = node_id_a.prv.bytes.data ();
	params.allow_bootstrap = allow_bootstrap_a;
	handle = rsnano::rsn_bootstrap_server_create (&params);
	debug_assert (socket_a != nullptr);
}

nano::transport::tcp_server::tcp_server (rsnano::TcpServerHandle * handle_a) :
	handle{ handle_a }
{
}

nano::transport::tcp_server::~tcp_server ()
{
	rsnano::rsn_bootstrap_server_destroy (handle);
}

void nano::transport::tcp_server::start ()
{
	rsnano::rsn_bootstrap_server_start (handle);
}

void nano::transport::tcp_server::stop ()
{
	rsnano::rsn_bootstrap_server_stop (handle);
}

/*
 * Bootstrap
 */

nano::transport::tcp_server::bootstrap_message_visitor::bootstrap_message_visitor (std::shared_ptr<tcp_server> server, std::shared_ptr<nano::node> node_a) :
	server{ std::move (server) },
	node{ std::move (node_a) }
{
}

void nano::transport::tcp_server::bootstrap_message_visitor::bulk_pull (const nano::bulk_pull & message)
{
	if (node->flags.disable_bootstrap_bulk_pull_server ())
	{
		return;
	}

	if (node->config->logging.bulk_pull_logging ())
	{
		node->logger->try_log (boost::str (boost::format ("Received bulk pull for %1% down to %2%, maximum of %3% from %4%") % message.get_start ().to_string () % message.get_end ().to_string () % message.get_count () % server->get_remote_endpoint ()));
	}

	node->bootstrap_workers.push_task ([server = server, message = message, node = node] () {
		// TODO: Add completion callback to bulk pull server
		// TODO: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
		auto bulk_pull_server = std::make_shared<nano::bulk_pull_server> (node, server, std::make_unique<nano::bulk_pull> (message));
		bulk_pull_server->send_next ();
	});

	processed = true;
}

void nano::transport::tcp_server::bootstrap_message_visitor::bulk_pull_account (const nano::bulk_pull_account & message)
{
	if (node->flags.disable_bootstrap_bulk_pull_server ())
	{
		return;
	}

	if (node->config->logging.bulk_pull_logging ())
	{
		node->logger->try_log (boost::str (boost::format ("Received bulk pull account for %1% with a minimum amount of %2%") % message.get_account ().to_account () % nano::amount (message.get_minimum_amount ()).format_balance (nano::Mxrb_ratio, 10, true)));
	}

	node->bootstrap_workers.push_task ([server = server, message = message, node = node] () {
		// TODO: Add completion callback to bulk pull server
		// TODO: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
		auto bulk_pull_account_server = std::make_shared<nano::bulk_pull_account_server> (node, server, std::make_unique<nano::bulk_pull_account> (message));
		bulk_pull_account_server->send_frontier ();
	});

	processed = true;
}

void nano::transport::tcp_server::bootstrap_message_visitor::bulk_push (const nano::bulk_push &)
{
	node->bootstrap_workers.push_task ([server = server, node = node] () {
		// TODO: Add completion callback to bulk pull server
		auto bulk_push_server = std::make_shared<nano::bulk_push_server> (node, server);
		bulk_push_server->throttled_receive ();
	});

	processed = true;
}

void nano::transport::tcp_server::bootstrap_message_visitor::frontier_req (const nano::frontier_req & message)
{
	if (node->config->logging.bulk_pull_logging ())
	{
		node->logger->try_log (boost::str (boost::format ("Received frontier request for %1% with age %2%") % message.get_start ().to_string () % message.get_age ()));
	}

	node->bootstrap_workers.push_task ([server = server, message = message, node = node] () {
		// TODO: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
		auto response = std::make_shared<nano::frontier_req_server> (node, server, std::make_unique<nano::frontier_req> (message));
		response->send_next ();
	});

	processed = true;
}

// TODO: We could periodically call this (from a dedicated timeout thread for eg.) but socket already handles timeouts,
//  and since we only ever store tcp_server as weak_ptr, socket timeout will automatically trigger tcp_server cleanup
void nano::transport::tcp_server::timeout ()
{
	rsnano::rsn_bootstrap_server_timeout (handle);
}

nano::transport::request_response_visitor_factory::request_response_visitor_factory (nano::node & node_a) :
	node{ node_a }
{
}

std::shared_ptr<nano::message_visitor> nano::transport::request_response_visitor_factory::create_bootstrap (std::shared_ptr<nano::transport::tcp_server> connection_a)
{
	return std::make_shared<nano::transport::tcp_server::bootstrap_message_visitor> (connection_a, node.shared ());
}

bool nano::transport::tcp_server::is_stopped () const
{
	return rsnano::rsn_bootstrap_server_is_stopped (handle);
}

std::uintptr_t nano::transport::tcp_server::unique_id () const
{
	return rsnano::rsn_bootstrap_server_unique_id (handle);
}

void nano::transport::tcp_server::set_remote_node_id (nano::account account_a)
{
	rsnano::rsn_bootstrap_server_set_remote_node_id (handle, account_a.bytes.data ());
}

nano::tcp_endpoint nano::transport::tcp_server::get_remote_endpoint () const
{
	rsnano::EndpointDto dto;
	rsnano::rsn_bootstrap_server_remote_endpoint (handle, &dto);
	return rsnano::dto_to_endpoint (dto);
}

std::shared_ptr<nano::socket> const nano::transport::tcp_server::get_socket () const
{
	auto socket_handle = rsnano::rsn_bootstrap_server_socket (handle);
	return std::make_shared<nano::socket> (socket_handle);
}
