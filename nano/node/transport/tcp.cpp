#include "nano/lib/rsnano.hpp"
#include "nano/lib/rsnanoutils.hpp"
#include "nano/node/transport/channel.hpp"
#include "nano/node/transport/traffic_type.hpp"
#include "nano/secure/common.hpp"

#include <nano/lib/config.hpp>
#include <nano/lib/stats.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/fake.hpp>
#include <nano/node/transport/inproc.hpp>
#include <nano/node/transport/tcp.hpp>

#include <boost/format.hpp>

#include <cstdint>
#include <memory>
#include <stdexcept>

/*
 * tcp_message_manager
 */

nano::tcp_message_manager::tcp_message_manager (unsigned incoming_connections_max_a) :
	handle{ rsnano::rsn_tcp_message_manager_create (incoming_connections_max_a) }
{
}

nano::tcp_message_manager::~tcp_message_manager ()
{
	rsnano::rsn_tcp_message_manager_destroy (handle);
}

void nano::tcp_message_manager::put_message (nano::tcp_message_item const & item_a)
{
	rsnano::rsn_tcp_message_manager_put_message (handle, item_a.handle);
}

nano::tcp_message_item nano::tcp_message_manager::get_message ()
{
	return nano::tcp_message_item{ rsnano::rsn_tcp_message_manager_get_message (handle) };
}

void nano::tcp_message_manager::stop ()
{
	rsnano::rsn_tcp_message_manager_stop (handle);
}

/*
 * channel_tcp
 */

nano::transport::channel_tcp::channel_tcp (boost::asio::io_context & io_ctx_a, nano::outbound_bandwidth_limiter & limiter_a, nano::network_constants const & network_a, std::shared_ptr<nano::transport::socket> const & socket_a, std::shared_ptr<nano::transport::channel_tcp_observer> const & observer_a, size_t channel_id) :
	channel (rsnano::rsn_channel_tcp_create (
	std::chrono::steady_clock::now ().time_since_epoch ().count (),
	socket_a->handle,
	new std::weak_ptr<nano::transport::channel_tcp_observer> (observer_a),
	limiter_a.handle,
	&io_ctx_a,
	channel_id))
{
	set_network_version (network_a.protocol_version);
}

uint8_t nano::transport::channel_tcp::get_network_version () const
{
	return rsnano::rsn_channel_tcp_network_version (handle);
}

void nano::transport::channel_tcp::set_network_version (uint8_t network_version_a)
{
	rsnano::rsn_channel_tcp_network_set_version (handle, network_version_a);
}

nano::tcp_endpoint nano::transport::channel_tcp::get_tcp_endpoint () const
{
	rsnano::EndpointDto ep_dto{};
	rsnano::rsn_channel_tcp_endpoint (handle, &ep_dto);
	return rsnano::dto_to_endpoint (ep_dto);
}

bool nano::transport::channel_tcp::max (nano::transport::traffic_type traffic_type)
{
	return rsnano::rsn_channel_tcp_max (handle, static_cast<uint8_t> (traffic_type));
}

std::size_t nano::transport::channel_tcp::hash_code () const
{
	std::hash<::nano::tcp_endpoint> hash;
	return hash (get_tcp_endpoint ());
}

bool nano::transport::channel_tcp::operator== (nano::transport::channel const & other_a) const
{
	bool result (false);
	auto other_l (dynamic_cast<nano::transport::channel_tcp const *> (&other_a));
	if (other_l != nullptr)
	{
		return *this == *other_l;
	}
	return result;
}

void nano::transport::channel_tcp_send_callback (void * context_a, const rsnano::ErrorCodeDto * ec_a, std::size_t size_a)
{
	auto callback_ptr = static_cast<std::function<void (boost::system::error_code const &, std::size_t)> *> (context_a);
	if (*callback_ptr)
	{
		auto ec{ rsnano::dto_to_error_code (*ec_a) };
		(*callback_ptr) (ec, size_a);
	}
}

void nano::transport::delete_send_buffer_callback (void * context_a)
{
	auto callback_ptr = static_cast<std::function<void (boost::system::error_code const &, std::size_t)> *> (context_a);
	delete callback_ptr;
}

void nano::transport::channel_tcp::send (nano::message & message_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::transport::buffer_drop_policy drop_policy_a, nano::transport::traffic_type traffic_type)
{
	auto callback_pointer = new std::function<void (boost::system::error_code const &, std::size_t)> (callback_a);
	rsnano::rsn_channel_tcp_send (handle, message_a.handle, nano::transport::channel_tcp_send_callback, nano::transport::delete_send_buffer_callback, callback_pointer, static_cast<uint8_t> (drop_policy_a), static_cast<uint8_t> (traffic_type));
}

void nano::transport::channel_tcp::send_buffer (nano::shared_const_buffer const & buffer_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::transport::buffer_drop_policy policy_a, nano::transport::traffic_type traffic_type)
{
	auto callback_pointer = new std::function<void (boost::system::error_code const &, std::size_t)> (callback_a);
	rsnano::rsn_channel_tcp_send_buffer (handle, buffer_a.data (), buffer_a.size (), nano::transport::channel_tcp_send_callback, nano::transport::delete_send_buffer_callback, callback_pointer, static_cast<uint8_t> (policy_a), static_cast<uint8_t> (traffic_type));
}

std::string nano::transport::channel_tcp::to_string () const
{
	return boost::str (boost::format ("%1%") % get_tcp_endpoint ());
}

bool nano::transport::channel_tcp::operator== (nano::transport::channel_tcp const & other_a) const
{
	return rsnano::rsn_channel_tcp_eq (handle, other_a.handle);
}

std::shared_ptr<nano::transport::socket> nano::transport::channel_tcp::try_get_socket () const
{
	auto socket_handle{ rsnano::rsn_channel_tcp_socket (handle) };
	std::shared_ptr<nano::transport::socket> socket;
	if (socket_handle)
	{
		socket = std::make_shared<nano::transport::socket> (socket_handle);
	}
	return socket;
}

void nano::transport::channel_tcp::set_endpoint ()
{
	rsnano::rsn_channel_tcp_set_endpoint (handle);
}

nano::endpoint nano::transport::channel_tcp::get_peering_endpoint () const
{
	rsnano::EndpointDto dto;
	rsnano::rsn_channel_tcp_peering_endpoint (handle, &dto);
	return rsnano::dto_to_udp_endpoint (dto);
}

void nano::transport::channel_tcp::set_peering_endpoint (nano::endpoint endpoint)
{
	auto dto{ rsnano::udp_endpoint_to_dto (endpoint) };
	rsnano::rsn_channel_tcp_set_peering_endpoint (handle, &dto);
}

bool nano::transport::channel_tcp::alive () const
{
	return rsnano::rsn_channel_tcp_is_alive (handle);
}

nano::transport::tcp_server_factory::tcp_server_factory (nano::node & node) :
	node{ node }
{
}

std::shared_ptr<nano::transport::tcp_server> nano::transport::tcp_server_factory::create_tcp_server (const std::shared_ptr<nano::transport::channel_tcp> & channel_a, const std::shared_ptr<nano::transport::socket> & socket_a)
{
	channel_a->set_last_packet_sent (std::chrono::steady_clock::now ());

	auto response_server = std::make_shared<nano::transport::tcp_server> (
	node.io_ctx,
	socket_a,
	node.logger,
	*node.stats,
	node.flags,
	*node.config,
	node.tcp_listener,
	std::make_shared<nano::transport::request_response_visitor_factory> (node),
	node.bootstrap_workers,
	*node.network->tcp_channels->publish_filter,
	node.block_uniquer,
	node.vote_uniquer,
	node.network->tcp_channels->tcp_message_manager,
	*node.network->syn_cookies,
	node.ledger,
	node.block_processor,
	node.bootstrap_initiator,
	node.node_id,
	true);

	// Listen for possible responses
	response_server->get_socket ()->type_set (nano::transport::socket::type_t::realtime_response_server);
	response_server->set_remote_node_id (channel_a->get_node_id ());
	response_server->start ();

	return response_server;
}

/*
 * tcp_channels
 */

nano::transport::tcp_channels::tcp_channels (nano::node & node, uint16_t port, std::function<void (nano::message const &, std::shared_ptr<nano::transport::channel> const &)> sink) :
	tcp_message_manager{ node.config->tcp_incoming_connections_max },
	tcp_server_factory{ node },
	node_id{ node.node_id },
	network_params{ node.network_params },
	limiter{ node.outbound_limiter },
	syn_cookies{ node.network->syn_cookies },
	stats{ node.stats },
	config{ node.config },
	logger{ node.logger },
	workers{ node.workers },
	flags{ node.flags },
	store{ node.store },
	io_ctx{ node.io_ctx },
	observers{ node.observers },
	sink{ std::move (sink) },
	node{ node },
	port{ port },
	publish_filter{ std::make_shared<nano::network_filter> (256 * 1024) },
	handle{ rsnano::rsn_tcp_channels_create () }
{
}

nano::transport::tcp_channels::~tcp_channels ()
{
	rsnano::rsn_tcp_channels_destroy (handle);
}

bool nano::transport::tcp_channels::insert (std::shared_ptr<nano::transport::channel_tcp> const & channel_a, std::shared_ptr<nano::transport::socket> const & socket_a, std::shared_ptr<nano::transport::tcp_server> const & server_a)
{
	auto endpoint (channel_a->get_tcp_endpoint ());
	debug_assert (endpoint.address ().is_v6 ());
	auto udp_endpoint (nano::transport::map_tcp_to_endpoint (endpoint));
	bool error (true);
	if (!not_a_peer (udp_endpoint, config->allow_local_peers) && !stopped)
	{
		nano::unique_lock<nano::mutex> lock{ mutex };
		auto existing (channels.get<endpoint_tag> ().find (endpoint));
		if (existing == channels.get<endpoint_tag> ().end ())
		{
			auto node_id (channel_a->get_node_id ());
			if (!channel_a->is_temporary ())
			{
				channels.get<node_id_tag> ().erase (node_id);
			}
			channels.get<endpoint_tag> ().emplace (channel_a, socket_a, server_a);
			attempts.get<endpoint_tag> ().erase (endpoint);
			error = false;
			lock.unlock ();
			channel_observer (channel_a);
		}
	}
	return error;
}

void nano::transport::tcp_channels::erase (nano::tcp_endpoint const & endpoint_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	channels.get<endpoint_tag> ().erase (endpoint_a);
}

void nano::transport::tcp_channels::erase_temporary_channel (nano::tcp_endpoint const & endpoint_a)
{
	auto exisiting_response_channel (find_channel (endpoint_a));
	if (exisiting_response_channel != nullptr)
	{
		exisiting_response_channel->set_temporary (false);
		erase (endpoint_a);
	}
}

std::size_t nano::transport::tcp_channels::size () const
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	return channels.size ();
}

std::shared_ptr<nano::transport::channel_tcp> nano::transport::tcp_channels::find_channel (nano::tcp_endpoint const & endpoint_a) const
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	std::shared_ptr<nano::transport::channel_tcp> result;
	auto existing (channels.get<endpoint_tag> ().find (endpoint_a));
	if (existing != channels.get<endpoint_tag> ().end ())
	{
		result = existing->get_channel ();
	}
	return result;
}

std::unordered_set<std::shared_ptr<nano::transport::channel>> nano::transport::tcp_channels::random_set (std::size_t count_a, uint8_t min_version, bool include_temporary_channels_a) const
{
	std::unordered_set<std::shared_ptr<nano::transport::channel>> result;
	result.reserve (count_a);
	nano::lock_guard<nano::mutex> lock{ mutex };
	// Stop trying to fill result with random samples after this many attempts
	auto random_cutoff (count_a * 2);
	auto peers_size (channels.size ());
	// Usually count_a will be much smaller than peers.size()
	// Otherwise make sure we have a cutoff on attempting to randomly fill
	if (!channels.empty ())
	{
		for (auto i (0); i < random_cutoff && result.size () < count_a; ++i)
		{
			auto index (nano::random_pool::generate_word32 (0, static_cast<uint32_t> (peers_size - 1)));

			auto channel = channels.get<random_access_tag> ()[index].get_channel ();
			if (!channel->alive ())
			{
				continue;
			}

			if (channel->get_network_version () >= min_version && (include_temporary_channels_a || !channel->is_temporary ()))
			{
				result.insert (channel);
			}
		}
	}
	return result;
}

std::vector<nano::endpoint> nano::transport::tcp_channels::get_peers () const
{
	// We can't hold the mutex while starting a write transaction, so
	// we collect endpoints to be saved and then relase the lock.
	nano::lock_guard<nano::mutex> lock{ mutex };
	std::vector<nano::endpoint> endpoints;
	endpoints.reserve (channels.size ());
	std::transform (channels.begin (), channels.end (),
	std::back_inserter (endpoints), [] (auto const & channel) { return nano::transport::map_tcp_to_endpoint (channel.endpoint ()); });
	return endpoints;
}

void nano::transport::tcp_channels::random_fill (std::array<nano::endpoint, 8> & target_a) const
{
	auto peers (random_set (target_a.size (), 0, false)); // Don't include channels with ephemeral remote ports
	debug_assert (peers.size () <= target_a.size ());
	auto endpoint (nano::endpoint (boost::asio::ip::address_v6{}, 0));
	debug_assert (endpoint.address ().is_v6 ());
	std::fill (target_a.begin (), target_a.end (), endpoint);
	auto j (target_a.begin ());
	for (auto i (peers.begin ()), n (peers.end ()); i != n; ++i, ++j)
	{
		debug_assert ((*i)->get_peering_endpoint ().address ().is_v6 ());
		debug_assert (j < target_a.end ());
		*j = (*i)->get_peering_endpoint ();
	}
}

void nano::transport::tcp_channels::set_port (uint16_t port_a)
{
	port = port_a;
}

std::vector<nano::endpoint> nano::transport::tcp_channels::get_current_peers () const
{
	std::vector<nano::endpoint> endpoints;
	nano::lock_guard<nano::mutex> lock{ mutex };
	endpoints.reserve (channels.size ());
	std::transform (channels.begin (), channels.end (),
	std::back_inserter (endpoints), [] (auto const & channel) { return nano::transport::map_tcp_to_endpoint (channel.endpoint ()); });
	return endpoints;
}

std::shared_ptr<nano::transport::channel_tcp> nano::transport::tcp_channels::find_node_id (nano::account const & node_id_a)
{
	std::shared_ptr<nano::transport::channel_tcp> result;
	nano::lock_guard<nano::mutex> lock{ mutex };
	auto existing (channels.get<node_id_tag> ().find (node_id_a));
	if (existing != channels.get<node_id_tag> ().end ())
	{
		result = existing->get_channel ();
	}
	return result;
}

nano::tcp_endpoint nano::transport::tcp_channels::bootstrap_peer ()
{
	nano::tcp_endpoint result (boost::asio::ip::address_v6::any (), 0);
	nano::lock_guard<nano::mutex> lock{ mutex };
	for (auto i (channels.get<last_bootstrap_attempt_tag> ().begin ()), n (channels.get<last_bootstrap_attempt_tag> ().end ()); i != n;)
	{
		if (i->get_channel ()->get_network_version () >= network_params.network.protocol_version_min)
		{
			result = nano::transport::map_endpoint_to_tcp (i->get_channel ()->get_peering_endpoint ());
			channels.get<last_bootstrap_attempt_tag> ().modify (i, [] (channel_tcp_wrapper & wrapper_a) {
				wrapper_a.get_channel ()->set_last_bootstrap_attempt (std::chrono::steady_clock::now ());
			});
			i = n;
		}
		else
		{
			++i;
		}
	}
	return result;
}

void nano::transport::tcp_channels::process_messages ()
{
	while (!stopped)
	{
		auto item (tcp_message_manager.get_message ());
		if (item.get_message () != nullptr)
		{
			auto message{ item.get_message () };
			process_message (*message, item.get_endpoint (), item.get_node_id (), item.get_socket ());
		}
	}
}

void nano::transport::tcp_channels::process_message (nano::message const & message_a, nano::tcp_endpoint const & endpoint_a, nano::account const & node_id_a, std::shared_ptr<nano::transport::socket> const & socket_a)
{
	auto type_a = socket_a->type ();
	if (!stopped && message_a.get_header ().get_version_using () >= network_params.network.protocol_version_min)
	{
		auto channel (find_channel (endpoint_a));
		if (channel)
		{
			sink (message_a, channel);
		}
		else
		{
			channel = find_node_id (node_id_a);
			if (channel)
			{
				sink (message_a, channel);
			}
			else if (!excluded_peers.check (endpoint_a))
			{
				if (!node_id_a.is_zero ())
				{
					// Add temporary channel
					auto temporary_channel (std::make_shared<nano::transport::channel_tcp> (io_ctx, limiter, config->network_params.network, socket_a, shared_from_this (), next_channel_id.fetch_add (1)));
					temporary_channel->set_endpoint ();
					debug_assert (endpoint_a == temporary_channel->get_tcp_endpoint ());
					temporary_channel->set_node_id (node_id_a);
					temporary_channel->set_network_version (message_a.get_header ().get_version_using ());
					temporary_channel->set_temporary (true);
					debug_assert (type_a == nano::transport::socket::type_t::realtime || type_a == nano::transport::socket::type_t::realtime_response_server);
					// Don't insert temporary channels for response_server
					if (type_a == nano::transport::socket::type_t::realtime)
					{
						insert (temporary_channel, socket_a, nullptr);
					}
					sink (message_a, temporary_channel);
				}
				else
				{
					// Initial node_id_handshake request without node ID
					debug_assert (message_a.get_header ().get_type () == nano::message_type::node_id_handshake);
					stats->inc (nano::stat::type::message, nano::stat::detail::node_id_handshake, nano::stat::dir::in);
				}
			}
		}
		if (channel)
		{
			channel->set_last_packet_received (std::chrono::steady_clock::now ());
		}
	}
}

void nano::transport::tcp_channels::start ()
{
	ongoing_keepalive ();
}

void nano::transport::tcp_channels::stop ()
{
	stopped = true;
	nano::unique_lock<nano::mutex> lock{ mutex };
	// Close all TCP sockets
	for (auto const & channel : channels)
	{
		auto socket{ channel.try_get_socket () };
		if (socket)
		{
			socket->close ();
		}
		// Remove response server
		auto server{ channel.get_response_server () };
		if (server)
		{
			server->stop ();
		}
	}
	channels.clear ();
	tcp_message_manager.stop ();
}

bool nano::transport::tcp_channels::not_a_peer (nano::endpoint const & endpoint_a, bool allow_local_peers)
{
	bool result (false);
	if (endpoint_a.address ().to_v6 ().is_unspecified ())
	{
		result = true;
	}
	else if (nano::transport::reserved_address (endpoint_a, allow_local_peers))
	{
		result = true;
	}
	else if (endpoint_a == nano::endpoint (boost::asio::ip::address_v6::loopback (), port))
	{
		result = true;
	}
	return result;
}

bool nano::transport::tcp_channels::max_ip_connections (nano::tcp_endpoint const & endpoint_a)
{
	if (flags.disable_max_peers_per_ip ())
	{
		return false;
	}
	bool result{ false };
	auto const address (nano::transport::ipv4_address_or_ipv6_subnet (endpoint_a.address ()));
	nano::unique_lock<nano::mutex> lock{ mutex };
	result = channels.get<ip_address_tag> ().count (address) >= network_params.network.max_peers_per_ip;
	if (!result)
	{
		result = attempts.get<ip_address_tag> ().count (address) >= network_params.network.max_peers_per_ip;
	}
	if (result)
	{
		stats->inc (nano::stat::type::tcp, nano::stat::detail::tcp_max_per_ip, nano::stat::dir::out);
	}
	return result;
}

bool nano::transport::tcp_channels::max_subnetwork_connections (nano::tcp_endpoint const & endpoint_a)
{
	if (flags.disable_max_peers_per_subnetwork ())
	{
		return false;
	}
	bool result{ false };
	auto const subnet (nano::transport::map_address_to_subnetwork (endpoint_a.address ()));
	nano::unique_lock<nano::mutex> lock{ mutex };
	result = channels.get<subnetwork_tag> ().count (subnet) >= network_params.network.max_peers_per_subnetwork;
	if (!result)
	{
		result = attempts.get<subnetwork_tag> ().count (subnet) >= network_params.network.max_peers_per_subnetwork;
	}
	if (result)
	{
		stats->inc (nano::stat::type::tcp, nano::stat::detail::tcp_max_per_subnetwork, nano::stat::dir::out);
	}
	return result;
}

bool nano::transport::tcp_channels::max_ip_or_subnetwork_connections (nano::tcp_endpoint const & endpoint_a)
{
	return max_ip_connections (endpoint_a) || max_subnetwork_connections (endpoint_a);
}

bool nano::transport::tcp_channels::reachout (nano::endpoint const & endpoint_a)
{
	auto tcp_endpoint (nano::transport::map_endpoint_to_tcp (endpoint_a));
	// Don't overload single IP
	bool error = excluded_peers.check (tcp_endpoint) || max_ip_or_subnetwork_connections (tcp_endpoint);
	if (!error && !flags.disable_tcp_realtime ())
	{
		// Don't keepalive to nodes that already sent us something
		error |= find_channel (tcp_endpoint) != nullptr;
		nano::lock_guard<nano::mutex> lock{ mutex };
		auto inserted (attempts.emplace (tcp_endpoint));
		error |= !inserted.second;
	}
	return error;
}

std::unique_ptr<nano::container_info_component> nano::transport::tcp_channels::collect_container_info (std::string const & name)
{
	std::size_t channels_count;
	std::size_t attemps_count;
	{
		nano::lock_guard<nano::mutex> guard{ mutex };
		channels_count = channels.size ();
		attemps_count = attempts.size ();
	}

	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "channels", channels_count, sizeof (decltype (channels)::value_type) }));
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "attempts", attemps_count, sizeof (decltype (attempts)::value_type) }));

	return composite;
}

void nano::transport::tcp_channels::purge (std::chrono::steady_clock::time_point const & cutoff_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };

	// Remove channels with dead underlying sockets
	for (auto it = channels.begin (); it != channels.end (); ++it)
	{
		if (!it->get_channel ()->alive ())
		{
			it = channels.erase (it);
		}
	}

	auto disconnect_cutoff (channels.get<last_packet_sent_tag> ().lower_bound (cutoff_a));
	channels.get<last_packet_sent_tag> ().erase (channels.get<last_packet_sent_tag> ().begin (), disconnect_cutoff);

	// Remove keepalive attempt tracking for attempts older than cutoff
	auto attempts_cutoff (attempts.get<last_attempt_tag> ().lower_bound (cutoff_a));
	attempts.get<last_attempt_tag> ().erase (attempts.get<last_attempt_tag> ().begin (), attempts_cutoff);

	// Check if any tcp channels belonging to old protocol versions which may still be alive due to async operations
	auto lower_bound = channels.get<version_tag> ().lower_bound (network_params.network.protocol_version_min);
	channels.get<version_tag> ().erase (channels.get<version_tag> ().begin (), lower_bound);
}

void nano::transport::tcp_channels::ongoing_keepalive ()
{
	nano::keepalive message{ network_params.network };
	auto peers{ message.get_peers () };
	random_fill (peers);
	message.set_peers (peers);
	nano::unique_lock<nano::mutex> lock{ mutex };
	// Wake up channels
	std::vector<std::shared_ptr<nano::transport::channel_tcp>> send_list;
	auto keepalive_sent_cutoff (channels.get<last_packet_sent_tag> ().lower_bound (std::chrono::steady_clock::now () - network_params.network.keepalive_period));
	for (auto i (channels.get<last_packet_sent_tag> ().begin ()); i != keepalive_sent_cutoff; ++i)
	{
		send_list.push_back (i->get_channel ());
	}
	lock.unlock ();
	for (auto & channel : send_list)
	{
		channel->send (message);
	}
	std::weak_ptr<nano::transport::tcp_channels> this_w (shared_from_this ());
	workers->add_timed_task (std::chrono::steady_clock::now () + network_params.network.keepalive_period, [this_w] () {
		if (auto this_l = this_w.lock ())
		{
			if (!this_l->stopped)
			{
				this_l->ongoing_keepalive ();
			}
		}
	});
}

void nano::transport::tcp_channels::list (std::deque<std::shared_ptr<nano::transport::channel>> & deque_a, uint8_t minimum_version_a, bool include_temporary_channels_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	// clang-format off
	nano::transform_if (channels.get<random_access_tag> ().begin (), channels.get<random_access_tag> ().end (), std::back_inserter (deque_a),
		[include_temporary_channels_a, minimum_version_a](auto & channel_a) { return channel_a.get_channel()->get_network_version () >= minimum_version_a && (include_temporary_channels_a || !channel_a.get_channel()->is_temporary ()); },
		[](auto const & channel) { return channel.get_channel(); });
	// clang-format on
}

void nano::transport::tcp_channels::modify (std::shared_ptr<nano::transport::channel_tcp> const & channel_a, std::function<void (std::shared_ptr<nano::transport::channel_tcp> const &)> modify_callback_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	auto existing (channels.get<endpoint_tag> ().find (channel_a->get_tcp_endpoint ()));
	if (existing != channels.get<endpoint_tag> ().end ())
	{
		channels.get<endpoint_tag> ().modify (existing, [modify_callback = std::move (modify_callback_a)] (channel_tcp_wrapper & wrapper_a) {
			modify_callback (wrapper_a.get_channel ());
		});
	}
}

void nano::transport::tcp_channels::update (nano::tcp_endpoint const & endpoint_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	auto existing (channels.get<endpoint_tag> ().find (endpoint_a));
	if (existing != channels.get<endpoint_tag> ().end ())
	{
		channels.get<endpoint_tag> ().modify (existing, [] (channel_tcp_wrapper & wrapper_a) {
			wrapper_a.get_channel ()->set_last_packet_sent (std::chrono::steady_clock::now ());
		});
	}
}

void nano::transport::tcp_channels::start_tcp (nano::endpoint const & endpoint_a)
{
	auto socket = std::make_shared<nano::transport::socket> (io_ctx, nano::transport::socket::endpoint_type_t::client, *stats, logger, workers,
	config->tcp_io_timeout,
	network_params.network.silent_connection_tolerance_time,
	network_params.network.idle_timeout,
	config->logging.network_timeout_logging (),
	observers);
	auto channel (std::make_shared<nano::transport::channel_tcp> (io_ctx, limiter, config->network_params.network, socket, shared_from_this (), next_channel_id.fetch_add (1)));
	auto network_consts = network_params.network;
	auto config_l = config;
	auto logger_l = logger;
	std::weak_ptr<nano::transport::tcp_channels> this_w = shared_from_this ();
	socket->async_connect (nano::transport::map_endpoint_to_tcp (endpoint_a),
	[channel, socket, endpoint_a, network_consts, config_l, logger_l, this_w] (boost::system::error_code const & ec) {
		if (auto this_l = this_w.lock ())
		{
			if (!ec && channel)
			{
				// TCP node ID handshake
				auto query = this_l->prepare_handshake_query (endpoint_a);
				nano::node_id_handshake message{ network_consts, query };

				if (config_l->logging.network_node_id_handshake_logging ())
				{
					logger_l->try_log (boost::str (boost::format ("Node ID handshake request sent with node ID %1% to %2%: query %3%") % this_l->node_id.pub.to_node_id () % endpoint_a % (query ? query->cookie.to_string () : "not set")));
				}

				channel->set_endpoint ();
				std::shared_ptr<std::vector<uint8_t>> receive_buffer (std::make_shared<std::vector<uint8_t>> ());
				receive_buffer->resize (256);
				channel->send (message, [this_w, channel, endpoint_a, receive_buffer, config_l, logger_l] (boost::system::error_code const & ec, std::size_t size_a) {
					if (auto this_l = this_w.lock ())
					{
						if (!ec)
						{
							this_l->start_tcp_receive_node_id (channel, endpoint_a, receive_buffer);
						}
						else
						{
							if (auto socket_l = channel->try_get_socket ())
							{
								socket_l->close ();
							}
							if (config_l->logging.network_node_id_handshake_logging ())
							{
								logger_l->try_log (boost::str (boost::format ("Error sending node_id_handshake to %1%: %2%") % endpoint_a % ec.message ()));
							}
						}
					}
				});
			}
			else
			{
				if (config_l->logging.network_logging ())
				{
					if (ec)
					{
						logger_l->try_log (boost::str (boost::format ("Error connecting to %1%: %2%") % endpoint_a % ec.message ()));
					}
					else
					{
						logger_l->try_log (boost::str (boost::format ("Error connecting to %1%") % endpoint_a));
					}
				}
			}
		}
	});
}

namespace
{
void message_received_callback (void * context, const rsnano::ErrorCodeDto * ec_dto, rsnano::MessageHandle * msg_handle)
{
	auto callback = static_cast<std::function<void (boost::system::error_code, std::unique_ptr<nano::message>)> *> (context);
	auto ec = rsnano::dto_to_error_code (*ec_dto);
	std::unique_ptr<nano::message> message;
	if (msg_handle != nullptr)
	{
		message = rsnano::message_handle_to_message (rsnano::rsn_message_clone (msg_handle));
	}
	(*callback) (ec, std::move (message));
}

void delete_callback_context (void * context)
{
	auto callback = static_cast<std::function<void (boost::system::error_code, std::unique_ptr<nano::message>)> *> (context);
	delete callback;
}
}

void nano::transport::tcp_channels::start_tcp_receive_node_id (std::shared_ptr<nano::transport::channel_tcp> const & channel_a, nano::endpoint const & endpoint_a, std::shared_ptr<std::vector<uint8_t>> const & receive_buffer_a)
{
	std::weak_ptr<nano::transport::tcp_channels> this_w (shared_from_this ());
	auto socket_l = channel_a->try_get_socket ();
	if (!socket_l)
	{
		return;
	}
	std::weak_ptr<nano::transport::channel_tcp> channel_w (channel_a);
	auto cleanup_node_id_handshake_socket = [channel_w, this_w] (nano::endpoint const & endpoint_a) {
		if (auto this_l = this_w.lock ())
		{
			if (auto channel_l = channel_w.lock ())
			{
				if (auto socket_l = channel_l->try_get_socket ())
				{
					socket_l->close ();
				}
			}
		}
	};

	auto callback_context = new std::function<void (boost::system::error_code ec, std::unique_ptr<nano::message>)> (
	[this_w, socket_l, channel_a, endpoint_a, cleanup_node_id_handshake_socket] (boost::system::error_code ec, std::unique_ptr<nano::message> message) {
		auto this_l = this_w.lock ();
		if (!this_l)
		{
			return;
		}
		if (ec || !channel_a)
		{
			if (this_l->config->logging.network_node_id_handshake_logging ())
			{
				this_l->logger->try_log (boost::str (boost::format ("Error reading node_id_handshake from %1%") % endpoint_a));
			}
			cleanup_node_id_handshake_socket (endpoint_a);
			return;
		}
		this_l->stats->inc (nano::stat::type::message, nano::stat::detail::node_id_handshake, nano::stat::dir::in);
		auto error (false);

		// the header type should in principle be checked after checking the network bytes and the version numbers, I will not change it here since the benefits do not outweight the difficulties
		if (error || message->type () != nano::message_type::node_id_handshake)
		{
			if (this_l->config->logging.network_node_id_handshake_logging ())
			{
				this_l->logger->try_log (boost::str (boost::format ("Error reading node_id_handshake message header from %1%") % endpoint_a));
			}
			cleanup_node_id_handshake_socket (endpoint_a);
			return;
		}
		auto & handshake = static_cast<nano::node_id_handshake &> (*message);

		if (message->get_header ().get_network () != this_l->network_params.network.current_network || message->get_header ().get_version_using () < this_l->network_params.network.protocol_version_min)
		{
			// error handling, either the networks bytes or the version is wrong
			if (message->get_header ().get_network () == this_l->network_params.network.current_network)
			{
				this_l->stats->inc (nano::stat::type::message, nano::stat::detail::invalid_network);
			}
			else
			{
				this_l->stats->inc (nano::stat::type::message, nano::stat::detail::outdated_version);
			}

			cleanup_node_id_handshake_socket (endpoint_a);
			// Cleanup attempt
			{
				nano::lock_guard<nano::mutex> lock{ this_l->mutex };
				this_l->attempts.get<endpoint_tag> ().erase (nano::transport::map_endpoint_to_tcp (endpoint_a));
			}
			return;
		}

		if (error || !handshake.get_response () || !handshake.get_query ())
		{
			if (this_l->config->logging.network_node_id_handshake_logging ())
			{
				this_l->logger->try_log (boost::str (boost::format ("Error reading node_id_handshake from %1%") % endpoint_a));
			}
			cleanup_node_id_handshake_socket (endpoint_a);
			return;
		}
		channel_a->set_network_version (handshake.get_header ().get_version_using ());

		debug_assert (handshake.get_query ());
		debug_assert (handshake.get_response ());

		auto const node_id = handshake.get_response ()->node_id;

		if (!this_l->verify_handshake_response (*handshake.get_response (), endpoint_a))
		{
			cleanup_node_id_handshake_socket (endpoint_a);
			return;
		}

		/* If node ID is known, don't establish new connection
		   Exception: temporary channels from tcp_server */
		auto existing_channel (this_l->find_node_id (node_id));
		if (existing_channel && !existing_channel->is_temporary ())
		{
			cleanup_node_id_handshake_socket (endpoint_a);
			return;
		}
		channel_a->set_node_id (node_id);
		channel_a->set_last_packet_received (std::chrono::steady_clock::now ());

		debug_assert (handshake.get_query ());
		auto response = this_l->prepare_handshake_response (*handshake.get_query (), handshake.is_v2 ());
		nano::node_id_handshake handshake_response (this_l->network_params.network, std::nullopt, response);

		if (this_l->config->logging.network_node_id_handshake_logging ())
		{
			this_l->logger->try_log (boost::str (boost::format ("Node ID handshake response sent with node ID %1% to %2%: query %3%") % this_l->node_id.pub.to_node_id () % endpoint_a % handshake.get_query ()->cookie.to_string ()));
		}

		channel_a->send (handshake_response, [this_w, channel_a, endpoint_a, cleanup_node_id_handshake_socket] (boost::system::error_code const & ec, std::size_t size_a) {
			auto this_l = this_w.lock ();
			if (!this_l)
			{
				return;
			}
			if (ec || !channel_a)
			{
				if (this_l->config->logging.network_node_id_handshake_logging ())
				{
					this_l->logger->try_log (boost::str (boost::format ("Error sending node_id_handshake to %1%: %2%") % endpoint_a % ec.message ()));
				}
				cleanup_node_id_handshake_socket (endpoint_a);
				return;
			}
			// Insert new node ID connection
			auto socket_l = channel_a->try_get_socket ();
			if (!socket_l)
			{
				return;
			}
			auto response_server = this_l->tcp_server_factory.create_tcp_server (channel_a, socket_l);
			this_l->insert (channel_a, socket_l, response_server);
		});
	});

	auto network_constants_dto{ network_params.network.to_dto () };
	rsnano::rsn_message_deserializer_read_socket (
	&network_constants_dto,
	publish_filter->handle,
	node.block_uniquer.handle,
	node.vote_uniquer.handle,
	socket_l->handle,
	message_received_callback,
	callback_context,
	delete_callback_context);
}

void nano::transport::tcp_channels::data_sent (boost::asio::ip::tcp::endpoint const & endpoint_a)
{
	update (endpoint_a);
}

void nano::transport::tcp_channels::host_unreachable ()
{
	stats->inc (nano::stat::type::error, nano::stat::detail::unreachable_host, nano::stat::dir::out);
}

void nano::transport::tcp_channels::message_sent (nano::message const & message_a)
{
	auto detail = nano::to_stat_detail (message_a.get_header ().get_type ());
	stats->inc (nano::stat::type::message, detail, nano::stat::dir::out);
}

void nano::transport::tcp_channels::message_dropped (nano::message const & message_a, std::size_t buffer_size_a)
{
	nano::transport::callback_visitor visitor;
	message_a.visit (visitor);
	stats->inc (nano::stat::type::drop, visitor.result, nano::stat::dir::out);
	if (config->logging.network_packet_logging ())
	{
		logger->always_log (boost::str (boost::format ("%1% of size %2% dropped") % stats->detail_to_string (visitor.result) % buffer_size_a));
	}
}

void nano::transport::tcp_channels::no_socket_drop ()
{
	stats->inc (nano::stat::type::tcp, nano::stat::detail::tcp_write_no_socket_drop, nano::stat::dir::out);
}

void nano::transport::tcp_channels::write_drop ()
{
	stats->inc (nano::stat::type::tcp, nano::stat::detail::tcp_write_drop, nano::stat::dir::out);
}

void nano::transport::tcp_channels::on_new_channel (std::function<void (std::shared_ptr<nano::transport::channel>)> observer_a)
{
	channel_observer = std::move (observer_a);
}

nano::transport::tcp_channels::channel_tcp_wrapper::channel_tcp_wrapper (std::shared_ptr<nano::transport::channel_tcp> channel_a, std::shared_ptr<nano::transport::socket> socket_a, std::shared_ptr<nano::transport::tcp_server> server_a) :
	channel{ channel_a },
	server{ server_a }
{
	rsnano::TcpServerHandle * server_handle = nullptr;
	if (server_a)
		server_handle = server_a->handle;
	handle = rsnano::rsn_channel_tcp_wrapper_create (channel_a->handle, socket_a->handle, server_handle);
}

nano::transport::tcp_channels::channel_tcp_wrapper::~channel_tcp_wrapper ()
{
	rsnano::rsn_channel_tcp_wrapper_destroy (handle);
}

std::shared_ptr<nano::transport::channel_tcp> nano::transport::tcp_channels::channel_tcp_wrapper::get_channel () const
{
	return channel;
}
std::shared_ptr<nano::transport::tcp_server> nano::transport::tcp_channels::channel_tcp_wrapper::get_response_server () const
{
	return server;
}

nano::tcp_message_item::tcp_message_item () :
	handle{ rsnano::rsn_tcp_message_item_empty () }
{
}

nano::tcp_message_item::~tcp_message_item ()
{
	if (handle)
		rsnano::rsn_tcp_message_item_destroy (handle);
}

nano::tcp_message_item::tcp_message_item (std::shared_ptr<nano::message> message_a, nano::tcp_endpoint endpoint_a, nano::account node_id_a, std::shared_ptr<nano::transport::socket> socket_a)
{
	rsnano::MessageHandle * message_handle = nullptr;
	if (message_a)
	{
		message_handle = message_a->handle;
	}

	rsnano::EndpointDto endpoint_dto{ rsnano::endpoint_to_dto (endpoint_a) };
	rsnano::SocketHandle * socket_handle = nullptr;
	if (socket_a)
	{
		socket_handle = socket_a->handle;
	}
	handle = rsnano::rsn_tcp_message_item_create (message_handle, &endpoint_dto, node_id_a.bytes.data (), socket_handle);
}

nano::tcp_message_item::tcp_message_item (nano::tcp_message_item const & other_a) :
	handle{ rsnano::rsn_tcp_message_item_clone (other_a.handle) }
{
}

nano::tcp_message_item::tcp_message_item (nano::tcp_message_item && other_a) noexcept :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::tcp_message_item::tcp_message_item (rsnano::TcpMessageItemHandle * handle_a) :
	handle{ handle_a }
{
}

std::shared_ptr<nano::message> nano::tcp_message_item::get_message () const
{
	auto message_handle = rsnano::rsn_tcp_message_item_message (handle);
	return nano::message_handle_to_message (message_handle);
}

nano::tcp_endpoint nano::tcp_message_item::get_endpoint () const
{
	rsnano::EndpointDto endpoint_dto;
	rsnano::rsn_tcp_message_item_endpoint (handle, &endpoint_dto);
	return rsnano::dto_to_endpoint (endpoint_dto);
}

nano::account nano::tcp_message_item::get_node_id () const
{
	nano::account node_id;
	rsnano::rsn_tcp_message_item_node_id (handle, node_id.bytes.data ());
	return node_id;
}

std::shared_ptr<nano::transport::socket> nano::tcp_message_item::get_socket () const
{
	auto socket_handle = rsnano::rsn_tcp_message_item_socket (handle);
	return std::make_shared<nano::transport::socket> (socket_handle);
}

nano::tcp_message_item & nano::tcp_message_item::operator= (tcp_message_item const & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_tcp_message_item_destroy (handle);
	handle = rsnano::rsn_tcp_message_item_clone (other_a.handle);
	return *this;
}
nano::tcp_message_item & nano::tcp_message_item::operator= (tcp_message_item && other_a)
{
	if (handle != nullptr)
		rsnano::rsn_tcp_message_item_destroy (handle);
	handle = other_a.handle;
	other_a.handle = nullptr;
	return *this;
}

std::shared_ptr<nano::transport::channel> nano::transport::channel_handle_to_channel (rsnano::ChannelHandle * handle)
{
	auto channel_type = static_cast<nano::transport::transport_type> (rsnano::rsn_channel_type (handle));
	switch (channel_type)
	{
		case nano::transport::transport_type::tcp:
			return make_shared<nano::transport::channel_tcp> (handle);
		case nano::transport::transport_type::loopback:
			return make_shared<nano::transport::inproc::channel> (handle);
		case nano::transport::transport_type::fake:
			return make_shared<nano::transport::fake::channel> (handle);
		default:
			throw std::runtime_error ("unknown transport type");
	}
}

std::optional<nano::node_id_handshake::query_payload> nano::transport::tcp_channels::prepare_handshake_query (const nano::endpoint & remote_endpoint)
{
	if (auto cookie = syn_cookies->assign (remote_endpoint); cookie)
	{
		nano::node_id_handshake::query_payload query{ *cookie };
		return query;
	}
	return std::nullopt;
}

bool nano::transport::tcp_channels::verify_handshake_response (const nano::node_id_handshake::response_payload & response, const nano::endpoint & remote_endpoint)
{
	// Prevent connection with ourselves
	if (response.node_id == node_id.pub)
	{
		stats->inc (nano::stat::type::handshake, nano::stat::detail::invalid_node_id);
		return false; // Fail
	}

	// Prevent mismatched genesis
	if (response.v2 && response.v2->genesis != network_params.ledger.genesis->hash ())
	{
		stats->inc (nano::stat::type::handshake, nano::stat::detail::invalid_genesis);
		return false; // Fail
	}

	auto cookie = syn_cookies->cookie (remote_endpoint);
	if (!cookie)
	{
		stats->inc (nano::stat::type::handshake, nano::stat::detail::missing_cookie);
		return false; // Fail
	}

	if (!rsnano::rsn_message_node_id_handshake_response_validate (
		cookie->bytes.data (),
		response.node_id.bytes.data (),
		response.signature.bytes.data (),
		response.v2 ? response.v2->salt.bytes.data () : nullptr,
		response.v2 ? response.v2->genesis.bytes.data () : nullptr))
	{
		stats->inc (nano::stat::type::handshake, nano::stat::detail::invalid_signature);
		return false; // Fail
	}

	stats->inc (nano::stat::type::handshake, nano::stat::detail::ok);
	return true; // OK
}

nano::node_id_handshake::response_payload nano::transport::tcp_channels::prepare_handshake_response (const nano::node_id_handshake::query_payload & query, bool v2) const
{
	auto genesis{ node.network_params.ledger.genesis->hash () };
	rsnano::HandshakeResponseDto result;
	rsnano::rsn_message_node_id_handshake_response_create (
	query.cookie.bytes.data (),
	node.node_id.prv.bytes.data (),
	genesis.bytes.data (),
	&result);

	nano::node_id_handshake::response_payload response{};
	std::copy (std::begin (result.node_id), std::end (result.node_id), std::begin (response.node_id.bytes));
	std::copy (std::begin (result.signature), std::end (result.signature), std::begin (response.signature.bytes));
	if (result.v2)
	{
		nano::node_id_handshake::response_payload::v2_payload response_v2{};
		std::copy (std::begin (result.salt), std::end (result.salt), std::begin (response_v2.salt.bytes));
		std::copy (std::begin (result.genesis), std::end (result.genesis), std::begin (response_v2.genesis.bytes));
		response.v2 = response_v2;
	}

	return response;
}
