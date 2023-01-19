#include <nano/node/bootstrap/bootstrap_bulk_push.hpp>
#include <nano/node/bootstrap/bootstrap_frontier.hpp>
#include <nano/node/bootstrap/bootstrap_legacy.hpp>
#include <nano/node/node.hpp>

#include <boost/format.hpp>

nano::bootstrap_attempt_legacy::bootstrap_attempt_legacy (std::shared_ptr<nano::node> const & node_a, uint64_t const incremental_id_a, std::string const & id_a, uint32_t const frontiers_age_a, nano::account const & start_account_a) :
	nano::bootstrap_attempt (node_a, nano::bootstrap_mode::legacy, incremental_id_a, id_a),
	node (node_a),
	frontiers_age (frontiers_age_a),
	start_account (start_account_a)
{
}

bool nano::bootstrap_attempt_legacy::consume_future (std::future<bool> & future_a)
{
	bool result;
	try
	{
		result = future_a.get ();
	}
	catch (std::future_error &)
	{
		result = true;
	}
	return result;
}

void nano::bootstrap_attempt_legacy::stop ()
{
	auto lock{ rsnano::rsn_bootstrap_attempt_lock (handle) };
	set_stopped ();
	rsnano::rsn_bootstrap_attempt_unlock (lock);
	rsnano::rsn_bootstrap_attempt_notifiy_all (handle);
	lock = rsnano::rsn_bootstrap_attempt_lock (handle);
	if (auto i = frontiers.lock ())
	{
		try
		{
			i->promise.set_value (true);
		}
		catch (std::future_error &)
		{
		}
	}
	if (auto i = push.lock ())
	{
		try
		{
			i->promise.set_value (true);
		}
		catch (std::future_error &)
		{
		}
	}
	rsnano::rsn_bootstrap_attempt_unlock (lock);
	node->bootstrap_initiator.clear_pulls (get_incremental_id ());
}

rsnano::BootstrapAttemptLockHandle * nano::bootstrap_attempt_legacy::request_push (rsnano::BootstrapAttemptLockHandle * lock_a)
{
	bool error (false);
	rsnano::rsn_bootstrap_attempt_unlock (lock_a);
	auto connection_l (node->bootstrap_initiator.connections->find_connection (endpoint_frontier_request));
	lock_a = rsnano::rsn_bootstrap_attempt_lock (handle);
	if (connection_l)
	{
		std::future<bool> future;
		{
			auto this_l = std::dynamic_pointer_cast<nano::bootstrap_attempt_legacy> (shared_from_this ());
			auto client = std::make_shared<nano::bulk_push_client> (this_l->node, connection_l, this_l);
			client->start ();
			push = client;
			future = client->promise.get_future ();
		}
		rsnano::rsn_bootstrap_attempt_unlock (lock_a);
		error = consume_future (future); // This is out of scope of `client' so when the last reference via boost::asio::io_context is lost and the client is destroyed, the future throws an exception.
		lock_a = rsnano::rsn_bootstrap_attempt_lock (handle);
	}
	if (node->config->logging.network_logging ())
	{
		node->logger->try_log ("Exiting bulk push client");
		if (error)
		{
			node->logger->try_log ("Bulk push client failed");
		}
	}
	return lock_a;
}

void nano::bootstrap_attempt_legacy::add_frontier (nano::pull_info const & pull_a)
{
	// Prevent incorrect or malicious pulls with frontier 0 insertion
	if (!pull_a.head.is_zero ())
	{
		auto lock{ rsnano::rsn_bootstrap_attempt_lock (handle) };
		frontier_pulls.push_back (pull_a);
		rsnano::rsn_bootstrap_attempt_unlock (lock);
	}
}

void nano::bootstrap_attempt_legacy::add_bulk_push_target (nano::block_hash const & head, nano::block_hash const & end)
{
	auto lock{ rsnano::rsn_bootstrap_attempt_lock (handle) };
	bulk_push_targets.emplace_back (head, end);
	rsnano::rsn_bootstrap_attempt_unlock (lock);
}

bool nano::bootstrap_attempt_legacy::request_bulk_push_target (std::pair<nano::block_hash, nano::block_hash> & current_target_a)
{
	auto lock{ rsnano::rsn_bootstrap_attempt_lock (handle) };
	auto empty (bulk_push_targets.empty ());
	if (!empty)
	{
		current_target_a = bulk_push_targets.back ();
		bulk_push_targets.pop_back ();
	}
	rsnano::rsn_bootstrap_attempt_unlock (lock);
	return empty;
}

void nano::bootstrap_attempt_legacy::set_start_account (nano::account const & start_account_a)
{
	// Add last account fron frontier request
	auto lock{ rsnano::rsn_bootstrap_attempt_lock (handle) };
	start_account = start_account_a;
	rsnano::rsn_bootstrap_attempt_unlock (lock);
}

bool nano::bootstrap_attempt_legacy::request_frontier (rsnano::BootstrapAttemptLockHandle ** lock_a, bool first_attempt)
{
	auto result (true);
	rsnano::rsn_bootstrap_attempt_unlock (*lock_a);
	auto connection_l (node->bootstrap_initiator.connections->connection (shared_from_this (), first_attempt));
	*lock_a = rsnano::rsn_bootstrap_attempt_lock (handle);
	if (connection_l && !get_stopped ())
	{
		endpoint_frontier_request = connection_l->get_tcp_endpoint ();
		std::future<bool> future;
		{
			auto this_l = std::dynamic_pointer_cast<nano::bootstrap_attempt_legacy> (shared_from_this ());
			auto client = std::make_shared<nano::frontier_req_client> (this_l->node, connection_l, this_l);
			client->run (start_account, frontiers_age, node->config->bootstrap_frontier_request_count);
			frontiers = client;
			future = client->promise.get_future ();
		}
		rsnano::rsn_bootstrap_attempt_unlock (*lock_a);
		result = consume_future (future); // This is out of scope of `client' so when the last reference via boost::asio::io_context is lost and the client is destroyed, the future throws an exception.
		*lock_a = rsnano::rsn_bootstrap_attempt_lock (handle);
		if (result)
		{
			frontier_pulls.clear ();
		}
		else
		{
			account_count = nano::narrow_cast<unsigned int> (frontier_pulls.size ());
			// Shuffle pulls
			release_assert (std::numeric_limits<uint32_t>::max () > frontier_pulls.size ());
			if (!frontier_pulls.empty ())
			{
				for (auto i = static_cast<uint32_t> (frontier_pulls.size () - 1); i > 0; --i)
				{
					auto k = nano::random_pool::generate_word32 (0, i);
					std::swap (frontier_pulls[i], frontier_pulls[k]);
				}
			}
			// Add to regular pulls
			while (!frontier_pulls.empty ())
			{
				auto pull (frontier_pulls.front ());
				rsnano::rsn_bootstrap_attempt_unlock (*lock_a);
				node->bootstrap_initiator.connections->add_pull (pull);
				*lock_a = rsnano::rsn_bootstrap_attempt_lock (handle);
				inc_pulling ();
				frontier_pulls.pop_front ();
			}
		}
		if (node->config->logging.network_logging ())
		{
			if (!result)
			{
				node->logger->try_log (boost::str (boost::format ("Completed frontier request, %1% out of sync accounts according to %2%") % account_count % connection_l->channel_string ()));
			}
			else
			{
				node->stats->inc (nano::stat::type::error, nano::stat::detail::frontier_req, nano::stat::dir::out);
			}
		}
	}
	return result;
}

rsnano::BootstrapAttemptLockHandle * nano::bootstrap_attempt_legacy::run_start (rsnano::BootstrapAttemptLockHandle * lock_a)
{
	set_frontiers_received (false);
	auto frontier_failure (true);
	uint64_t frontier_attempts (0);
	while (!get_stopped () && frontier_failure)
	{
		++frontier_attempts;
		frontier_failure = request_frontier (&lock_a, frontier_attempts == 1);
	}
	set_frontiers_received (true);
	return lock_a;
}

void nano::bootstrap_attempt_legacy::run ()
{
	debug_assert (get_started ());
	debug_assert (!node->flags.disable_legacy_bootstrap ());
	node->bootstrap_initiator.connections->populate_connections (false);
	auto lock{ rsnano::rsn_bootstrap_attempt_lock (handle) };
	lock = run_start (lock);
	while (still_pulling ())
	{
		while (still_pulling ())
		{
			// clang-format off
			while (!( get_stopped () || get_pulling () == 0 ))
			{
				rsnano::rsn_bootstrap_attempt_wait (handle, lock);
			}
		}
		// Flushing may resolve forks which can add more pulls
		node->logger->try_log ("Flushing unchecked blocks");
		rsnano::rsn_bootstrap_attempt_unlock (lock);
		node->block_processor.flush ();
		lock = rsnano::rsn_bootstrap_attempt_lock (handle);
		if (start_account.number () != std::numeric_limits<nano::uint256_t>::max ())
		{
			node->logger->try_log (boost::str (boost::format ("Finished flushing unchecked blocks, requesting new frontiers after %1%") % start_account.to_account ()));
			// Requesting new frontiers
			lock = run_start (lock);
		}
		else
		{
			node->logger->try_log ("Finished flushing unchecked blocks");
		}
	}
	if (!get_stopped ())
	{
		node->logger->try_log ("Completed legacy pulls");
		if (!node->flags.disable_bootstrap_bulk_push_client ())
		{
			lock = request_push (lock);
		}
		if (!get_stopped ())
		{
			node->unchecked_cleanup ();
		}
	}
	rsnano::rsn_bootstrap_attempt_unlock (lock);
	stop ();
	rsnano::rsn_bootstrap_attempt_notifiy_all (handle);
}

void nano::bootstrap_attempt_legacy::get_information (boost::property_tree::ptree & tree_a)
{
	auto lock{ rsnano::rsn_bootstrap_attempt_lock (handle) };
	tree_a.put ("frontier_pulls", std::to_string (frontier_pulls.size ()));
	tree_a.put ("frontiers_received", static_cast<bool> (get_frontiers_received ()));
	tree_a.put ("frontiers_age", std::to_string (frontiers_age));
	tree_a.put ("last_account", start_account.to_account ());
	rsnano::rsn_bootstrap_attempt_unlock (lock);
}
