#include "nano/lib/rsnanoutils.hpp"

#include <nano/node/bootstrap/bootstrap.hpp>
#include <nano/node/bootstrap/bootstrap_attempt.hpp>
#include <nano/node/bootstrap/bootstrap_connections.hpp>
#include <nano/node/bootstrap/bootstrap_lazy.hpp>
#include <nano/node/common.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/tcp.hpp>

#include <boost/format.hpp>

#include <memory>

constexpr double nano::bootstrap_limits::bootstrap_connection_scale_target_blocks;
constexpr double nano::bootstrap_limits::bootstrap_minimum_blocks_per_sec;
constexpr double nano::bootstrap_limits::bootstrap_minimum_termination_time_sec;
constexpr unsigned nano::bootstrap_limits::bootstrap_max_new_connections;
constexpr unsigned nano::bootstrap_limits::requeued_pulls_processed_blocks_factor;

nano::bootstrap_client::bootstrap_client (std::shared_ptr<nano::bootstrap_client_observer> const & observer_a, std::shared_ptr<nano::transport::channel_tcp> const & channel_a, std::shared_ptr<nano::socket> const & socket_a) :
	handle{ rsnano::rsn_bootstrap_client_create (new std::shared_ptr<nano::bootstrap_client_observer> (observer_a), channel_a->handle, socket_a->handle) }
{
}

nano::bootstrap_client::~bootstrap_client ()
{
	rsnano::rsn_bootstrap_client_destroy (handle);
}

double nano::bootstrap_client::sample_block_rate ()
{
	return rsnano::rsn_bootstrap_client_sample_block_rate (handle);
}

void nano::bootstrap_client::set_start_time ()
{
	rsnano::rsn_bootstrap_client_set_start_time (handle);
}

double nano::bootstrap_client::elapsed_seconds () const
{
	return rsnano::rsn_bootstrap_client_elapsed_seconds (handle);
}

void nano::bootstrap_client::stop (bool force)
{
	rsnano::rsn_bootstrap_client_stop (handle, force);
}

void nano::bootstrap_client::async_read (std::size_t size_a, std::function<void (boost::system::error_code const &, std::size_t)> callback_a)
{
	auto cb_wrapper = new std::function<void (boost::system::error_code const &, std::size_t)> ([callback = std::move (callback_a), this_l = shared_from_this ()] (boost::system::error_code const & ec, std::size_t size) {
		callback (ec, size);
	});
	rsnano::rsn_bootstrap_client_read (handle, size_a, nano::async_read_adapter, nano::async_read_delete_context, cb_wrapper);
}

uint8_t * nano::bootstrap_client::get_receive_buffer ()
{
	buffer.resize (rsnano::rsn_bootstrap_client_receive_buffer_size (handle));
	rsnano::rsn_bootstrap_client_receive_buffer (handle, buffer.data (), buffer.size ());
	return buffer.data ();
}

nano::tcp_endpoint nano::bootstrap_client::remote_endpoint () const
{
	rsnano::EndpointDto result;
	rsnano::rsn_bootstrap_client_remote_endpoint (handle, &result);
	return rsnano::dto_to_endpoint (result);
}

std::string nano::bootstrap_client::channel_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_bootstrap_client_channel_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

void nano::bootstrap_client::send (nano::message & message_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::buffer_drop_policy drop_policy_a, nano::bandwidth_limit_type limit_type_a)
{
	auto callback_pointer = new std::function<void (boost::system::error_code const &, std::size_t)> (callback_a);
	rsnano::rsn_bootstrap_client_send (handle, message_a.handle, nano::transport::channel_tcp_send_callback, nano::transport::delete_send_buffer_callback, callback_pointer, static_cast<uint8_t> (drop_policy_a), static_cast<uint8_t> (limit_type_a));
}

void nano::bootstrap_client::send_buffer (nano::shared_const_buffer const & buffer_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::buffer_drop_policy policy_a)
{
	auto callback_pointer = new std::function<void (boost::system::error_code const &, std::size_t)> (callback_a);
	rsnano::rsn_bootstrap_client_send_buffer (handle, buffer_a.data (), buffer_a.size (), nano::transport::channel_tcp_send_callback, nano::transport::delete_send_buffer_callback, callback_pointer, static_cast<uint8_t> (policy_a));
}

nano::tcp_endpoint nano::bootstrap_client::get_tcp_endpoint () const
{
	rsnano::EndpointDto dto;
	rsnano::rsn_bootstrap_client_tcp_endpoint (handle, &dto);
	return rsnano::dto_to_endpoint (dto);
}

void nano::bootstrap_client::close_socket ()
{
	rsnano::rsn_bootstrap_client_close_socket (handle);
}

void nano::bootstrap_client::set_timeout (std::chrono::seconds timeout_a)
{
	rsnano::rsn_bootstrap_client_set_timeout (handle, timeout_a.count ());
}

std::shared_ptr<nano::socket> nano::bootstrap_client::get_socket () const
{
	return std::make_shared<nano::socket> (rsnano::rsn_bootstrap_client_socket (handle));
}

uint64_t nano::bootstrap_client::inc_block_count ()
{
	return rsnano::rsn_bootstrap_client_inc_block_count (handle);
}

uint64_t nano::bootstrap_client::get_block_count () const
{
	return rsnano::rsn_bootstrap_client_block_count (handle);
}
double nano::bootstrap_client::get_block_rate () const
{
	return rsnano::rsn_bootstrap_client_block_rate (handle);
}
bool nano::bootstrap_client::get_pending_stop () const
{
	return rsnano::rsn_bootstrap_client_pending_stop (handle);
}
bool nano::bootstrap_client::get_hard_stop () const
{
	return rsnano::rsn_bootstrap_client_hard_stop (handle);
}

nano::bootstrap_connections::bootstrap_connections (nano::node & node_a) :
	node (node_a)
{
}

std::shared_ptr<nano::bootstrap_client> nano::bootstrap_connections::connection (std::shared_ptr<nano::bootstrap_attempt> const & attempt_a, bool use_front_connection)
{
	nano::unique_lock<nano::mutex> lock{ mutex };
	condition.wait (lock, [&stopped = stopped, &idle = idle, &new_connections_empty = new_connections_empty] { return stopped || !idle.empty () || new_connections_empty; });
	std::shared_ptr<nano::bootstrap_client> result;
	if (!stopped && !idle.empty ())
	{
		if (!use_front_connection)
		{
			result = idle.back ();
			idle.pop_back ();
		}
		else
		{
			result = idle.front ();
			idle.pop_front ();
		}
	}
	if (result == nullptr && connections_count == 0 && new_connections_empty && attempt_a != nullptr)
	{
		node.logger->try_log (boost::str (boost::format ("Bootstrap attempt stopped because there are no peers")));
		lock.unlock ();
		attempt_a->stop ();
	}
	return result;
}

void nano::bootstrap_connections::pool_connection (std::shared_ptr<nano::bootstrap_client> const & client_a, bool new_client, bool push_front)
{
	nano::unique_lock<nano::mutex> lock{ mutex };
	if (!stopped && !client_a->get_pending_stop () && !node.network->excluded_peers.check (client_a->get_tcp_endpoint ()))
	{
		client_a->set_timeout (node.network_params.network.idle_timeout);
		// Push into idle deque
		if (!push_front)
		{
			idle.push_back (client_a);
		}
		else
		{
			idle.push_front (client_a);
		}
		if (new_client)
		{
			clients.push_back (client_a);
		}
	}
	else
	{
		client_a->close_socket ();
	}
	lock.unlock ();
	condition.notify_all ();
}

void nano::bootstrap_connections::add_connection (nano::endpoint const & endpoint_a)
{
	connect_client (nano::tcp_endpoint (endpoint_a.address (), endpoint_a.port ()), true);
}

std::shared_ptr<nano::bootstrap_client> nano::bootstrap_connections::find_connection (nano::tcp_endpoint const & endpoint_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	std::shared_ptr<nano::bootstrap_client> result;
	for (auto i (idle.begin ()), end (idle.end ()); i != end && !stopped; ++i)
	{
		if ((*i)->get_tcp_endpoint () == endpoint_a)
		{
			result = *i;
			idle.erase (i);
			break;
		}
	}
	return result;
}

void nano::bootstrap_connections::connect_client (nano::tcp_endpoint const & endpoint_a, bool push_front)
{
	++connections_count;
	auto socket (std::make_shared<nano::socket> (node.io_ctx, nano::socket::endpoint_type_t::client, *node.stats, node.logger, node.workers,
	node.config->tcp_io_timeout,
	node.network_params.network.silent_connection_tolerance_time,
	node.config->logging.network_timeout_logging (),
	node.observers));
	auto this_l (shared_from_this ());
	socket->async_connect (endpoint_a,
	[this_l, socket, endpoint_a, push_front] (boost::system::error_code const & ec) {
		if (!ec)
		{
			if (this_l->node.config->logging.bulk_pull_logging ())
			{
				this_l->node.logger->try_log (boost::str (boost::format ("Connection established to %1%") % endpoint_a));
			}
			auto client (std::make_shared<nano::bootstrap_client> (this_l, std::make_shared<nano::transport::channel_tcp> (this_l->node.io_ctx, this_l->node.outbound_limiter, this_l->node.config->network_params.network, socket, this_l->node.network->tcp_channels), socket));
			this_l->connections_count++;
			this_l->pool_connection (client, true, push_front);
		}
		else
		{
			if (this_l->node.config->logging.network_logging ())
			{
				switch (ec.value ())
				{
					default:
						this_l->node.logger->try_log (boost::str (boost::format ("Error initiating bootstrap connection to %1%: %2%") % endpoint_a % ec.message ()));
						break;
					case boost::system::errc::connection_refused:
					case boost::system::errc::operation_canceled:
					case boost::system::errc::timed_out:
					case 995: // Windows The I/O operation has been aborted because of either a thread exit or an application request
					case 10061: // Windows No connection could be made because the target machine actively refused it
						break;
				}
			}
		}
		--this_l->connections_count;
	});
}

unsigned nano::bootstrap_connections::target_connections (std::size_t pulls_remaining, std::size_t attempts_count) const
{
	auto const attempts_factor = nano::narrow_cast<unsigned> (node.config->bootstrap_connections * attempts_count);
	if (attempts_factor >= node.config->bootstrap_connections_max)
	{
		return std::max (1U, node.config->bootstrap_connections_max);
	}

	// Only scale up to bootstrap_connections_max for large pulls.
	double step_scale = std::min (1.0, std::max (0.0, (double)pulls_remaining / nano::bootstrap_limits::bootstrap_connection_scale_target_blocks));
	double target = (double)attempts_factor + (double)(node.config->bootstrap_connections_max - attempts_factor) * step_scale;
	return std::max (1U, (unsigned)(target + 0.5f));
}

struct block_rate_cmp
{
	bool operator() (std::shared_ptr<nano::bootstrap_client> const & lhs, std::shared_ptr<nano::bootstrap_client> const & rhs) const
	{
		return lhs->get_block_rate () > rhs->get_block_rate ();
	}
};

void nano::bootstrap_connections::populate_connections (bool repeat)
{
	double rate_sum = 0.0;
	std::size_t num_pulls = 0;
	std::size_t attempts_count = node.bootstrap_initiator.attempts.size ();
	std::priority_queue<std::shared_ptr<nano::bootstrap_client>, std::vector<std::shared_ptr<nano::bootstrap_client>>, block_rate_cmp> sorted_connections;
	std::unordered_set<nano::tcp_endpoint> endpoints;
	{
		nano::unique_lock<nano::mutex> lock{ mutex };
		num_pulls = pulls.size ();
		std::deque<std::weak_ptr<nano::bootstrap_client>> new_clients;
		for (auto & c : clients)
		{
			if (auto client = c.lock ())
			{
				new_clients.push_back (client);
				endpoints.insert (client->remote_endpoint ());
				double elapsed_sec = client->elapsed_seconds ();
				auto blocks_per_sec = client->sample_block_rate ();
				rate_sum += blocks_per_sec;
				if (client->elapsed_seconds () > nano::bootstrap_limits::bootstrap_connection_warmup_time_sec && client->get_block_count () > 0)
				{
					sorted_connections.push (client);
				}
				// Force-stop the slowest peers, since they can take the whole bootstrap hostage by dribbling out blocks on the last remaining pull.
				// This is ~1.5kilobits/sec.
				if (elapsed_sec > nano::bootstrap_limits::bootstrap_minimum_termination_time_sec && blocks_per_sec < nano::bootstrap_limits::bootstrap_minimum_blocks_per_sec)
				{
					if (node.config->logging.bulk_pull_logging ())
					{
						node.logger->try_log (boost::str (boost::format ("Stopping slow peer %1% (elapsed sec %2%s > %3%s and %4% blocks per second < %5%)") % client->channel_string () % elapsed_sec % nano::bootstrap_limits::bootstrap_minimum_termination_time_sec % blocks_per_sec % nano::bootstrap_limits::bootstrap_minimum_blocks_per_sec));
					}

					client->stop (true);
					new_clients.pop_back ();
				}
			}
		}
		// Cleanup expired clients
		clients.swap (new_clients);
	}

	auto target = target_connections (num_pulls, attempts_count);

	// We only want to drop slow peers when more than 2/3 are active. 2/3 because 1/2 is too aggressive, and 100% rarely happens.
	// Probably needs more tuning.
	if (sorted_connections.size () >= (target * 2) / 3 && target >= 4)
	{
		// 4 -> 1, 8 -> 2, 16 -> 4, arbitrary, but seems to work well.
		auto drop = (int)roundf (sqrtf ((float)target - 2.0f));

		if (node.config->logging.bulk_pull_logging ())
		{
			node.logger->try_log (boost::str (boost::format ("Dropping %1% bulk pull peers, target connections %2%") % drop % target));
		}

		for (int i = 0; i < drop; i++)
		{
			auto client = sorted_connections.top ();

			if (node.config->logging.bulk_pull_logging ())
			{
				node.logger->try_log (boost::str (boost::format ("Dropping peer with block rate %1%, block count %2% (%3%) ") % client->get_block_rate () % client->get_block_count () % client->channel_string ()));
			}

			client->stop (false);
			sorted_connections.pop ();
		}
	}

	if (node.config->logging.bulk_pull_logging ())
	{
		node.logger->try_log (boost::str (boost::format ("Bulk pull connections: %1%, rate: %2% blocks/sec, bootstrap attempts %3%, remaining pulls: %4%") % connections_count.load () % (int)rate_sum % attempts_count % num_pulls));
	}

	if (connections_count < target && (attempts_count != 0 || new_connections_empty) && !stopped)
	{
		auto delta = std::min ((target - connections_count) * 2, nano::bootstrap_limits::bootstrap_max_new_connections);
		// TODO - tune this better
		// Not many peers respond, need to try to make more connections than we need.
		for (auto i = 0u; i < delta; i++)
		{
			auto endpoint (node.network->bootstrap_peer (true));
			if (endpoint != nano::tcp_endpoint (boost::asio::ip::address_v6::any (), 0) && (node.flags.allow_bootstrap_peers_duplicates () || endpoints.find (endpoint) == endpoints.end ()) && !node.network->excluded_peers.check (endpoint))
			{
				connect_client (endpoint);
				endpoints.insert (endpoint);
				nano::lock_guard<nano::mutex> lock{ mutex };
				new_connections_empty = false;
			}
			else if (connections_count == 0)
			{
				{
					nano::lock_guard<nano::mutex> lock{ mutex };
					new_connections_empty = true;
				}
				condition.notify_all ();
			}
		}
	}
	if (!stopped && repeat)
	{
		std::weak_ptr<nano::bootstrap_connections> this_w (shared_from_this ());
		node.workers->add_timed_task (std::chrono::steady_clock::now () + std::chrono::seconds (1), [this_w] () {
			if (auto this_l = this_w.lock ())
			{
				this_l->populate_connections ();
			}
		});
	}
}

void nano::bootstrap_connections::start_populate_connections ()
{
	if (!populate_connections_started.exchange (true))
	{
		populate_connections ();
	}
}

void nano::bootstrap_connections::add_pull (nano::pull_info const & pull_a)
{
	nano::pull_info pull (pull_a);
	node.bootstrap_initiator.cache.update_pull (pull);
	{
		nano::lock_guard<nano::mutex> lock{ mutex };
		pulls.push_back (pull);
	}
	condition.notify_all ();
}

void nano::bootstrap_connections::request_pull (nano::unique_lock<nano::mutex> & lock_a)
{
	lock_a.unlock ();
	auto connection_l (connection ());
	lock_a.lock ();
	if (connection_l != nullptr && !pulls.empty ())
	{
		std::shared_ptr<nano::bootstrap_attempt> attempt_l;
		nano::pull_info pull;
		// Search pulls with existing attempts
		while (attempt_l == nullptr && !pulls.empty ())
		{
			pull = pulls.front ();
			pulls.pop_front ();
			attempt_l = node.bootstrap_initiator.attempts.find (pull.bootstrap_id);
			// Check if lazy pull is obsolete (head was processed or head is 0 for destinations requests)
			if (auto lazy = std::dynamic_pointer_cast<nano::bootstrap_attempt_lazy> (attempt_l))
			{
				if (!pull.head.is_zero () && lazy->lazy_processed_or_exists (pull.head))
				{
					attempt_l->pull_finished ();
					attempt_l = nullptr;
				}
			}
		}
		if (attempt_l != nullptr)
		{
			auto node_l{ node.shared_from_this () };
			// The bulk_pull_client destructor attempt to requeue_pull which can cause a deadlock if this is the last reference
			// Dispatch request in an external thread in case it needs to be destroyed
			node.background ([node_l, connection_l, attempt_l, pull] () {
				auto client (std::make_shared<nano::bulk_pull_client> (node_l, connection_l, attempt_l, pull));
				client->request ();
			});
		}
	}
	else if (connection_l != nullptr)
	{
		// Reuse connection if pulls deque become empty
		lock_a.unlock ();
		pool_connection (connection_l);
		lock_a.lock ();
	}
}

void nano::bootstrap_connections::requeue_pull (nano::pull_info const & pull_a, bool network_error)
{
	auto pull (pull_a);
	if (!network_error)
	{
		++pull.attempts;
	}
	auto attempt_l (node.bootstrap_initiator.attempts.find (pull.bootstrap_id));
	if (attempt_l != nullptr)
	{
		auto lazy = std::dynamic_pointer_cast<nano::bootstrap_attempt_lazy> (attempt_l);
		attempt_l->inc_requeued_pulls ();
		if (lazy)
		{
			pull.count = lazy->lazy_batch_size ();
		}
		if (attempt_l->get_mode () == nano::bootstrap_mode::legacy && (pull.attempts < pull.retry_limit + (pull.processed / nano::bootstrap_limits::requeued_pulls_processed_blocks_factor)))
		{
			{
				nano::lock_guard<nano::mutex> lock{ mutex };
				pulls.push_front (pull);
			}
			attempt_l->pull_started ();
			condition.notify_all ();
		}
		else if (lazy && (pull.attempts <= pull.retry_limit + (pull.processed / node.network_params.bootstrap.lazy_max_pull_blocks)))
		{
			debug_assert (pull.account_or_head.as_block_hash () == pull.head);
			if (!lazy->lazy_processed_or_exists (pull.account_or_head.as_block_hash ()))
			{
				{
					nano::lock_guard<nano::mutex> lock{ mutex };
					pulls.push_back (pull);
				}
				attempt_l->pull_started ();
				condition.notify_all ();
			}
		}
		else
		{
			if (node.config->logging.bulk_pull_logging ())
			{
				node.logger->try_log (boost::str (boost::format ("Failed to pull account %1% or head block %2% down to %3% after %4% attempts and %5% blocks processed") % pull.account_or_head.to_account () % pull.account_or_head.to_string () % pull.end.to_string () % pull.attempts % pull.processed));
			}
			node.stats->inc (nano::stat::type::bootstrap, nano::stat::detail::bulk_pull_failed_account, nano::stat::dir::in);

			if (lazy && pull.processed > 0)
			{
				lazy->lazy_add (pull);
			}
			else if (attempt_l->get_mode () == nano::bootstrap_mode::legacy)
			{
				node.bootstrap_initiator.cache.add (pull);
			}
		}
	}
}

void nano::bootstrap_connections::clear_pulls (uint64_t bootstrap_id_a)
{
	{
		nano::lock_guard<nano::mutex> lock{ mutex };
		auto i (pulls.begin ());
		while (i != pulls.end ())
		{
			if (i->bootstrap_id == bootstrap_id_a)
			{
				i = pulls.erase (i);
			}
			else
			{
				++i;
			}
		}
	}
	condition.notify_all ();
}

void nano::bootstrap_connections::run ()
{
	start_populate_connections ();
	nano::unique_lock<nano::mutex> lock{ mutex };
	while (!stopped)
	{
		if (!pulls.empty ())
		{
			request_pull (lock);
		}
		else
		{
			condition.wait (lock);
		}
	}
	stopped = true;
	lock.unlock ();
	condition.notify_all ();
}

void nano::bootstrap_connections::stop ()
{
	nano::unique_lock<nano::mutex> lock{ mutex };
	stopped = true;
	lock.unlock ();
	condition.notify_all ();
	lock.lock ();
	for (auto const & i : clients)
	{
		if (auto client = i.lock ())
		{
			client->close_socket ();
		}
	}
	clients.clear ();
	idle.clear ();
}

void nano::bootstrap_connections::bootstrap_client_closed ()
{
	--connections_count;
}
