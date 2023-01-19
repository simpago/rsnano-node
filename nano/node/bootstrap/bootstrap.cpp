#include <nano/lib/threading.hpp>
#include <nano/node/bootstrap/bootstrap.hpp>
#include <nano/node/bootstrap/bootstrap_lazy.hpp>
#include <nano/node/bootstrap/bootstrap_legacy.hpp>
#include <nano/node/common.hpp>
#include <nano/node/node.hpp>

#include <algorithm>
#include <memory>

nano::bootstrap_initiator::bootstrap_initiator (nano::node & node_a) :
	node (node_a),
	handle{ rsnano::rsn_bootstrap_initiator_create (this) }
{
	connections = std::make_shared<nano::bootstrap_connections> (node);
	bootstrap_initiator_threads.push_back (boost::thread ([this] () {
		nano::thread_role::set (nano::thread_role::name::bootstrap_connections);
		connections->run ();
	}));
	for (std::size_t i = 0; i < node.config->bootstrap_initiator_threads; ++i)
	{
		bootstrap_initiator_threads.push_back (boost::thread ([this] () {
			nano::thread_role::set (nano::thread_role::name::bootstrap_initiator);
			run_bootstrap ();
		}));
	}
}

nano::bootstrap_initiator::~bootstrap_initiator ()
{
	stop ();
	rsnano::rsn_bootstrap_initiator_destroy (handle);
}

void nano::bootstrap_initiator::bootstrap (bool force, std::string id_a, uint32_t const frontiers_age_a, nano::account const & start_account_a)
{
	if (force)
	{
		stop_attempts ();
	}
	nano::unique_lock<nano::mutex> lock{ mutex };
	if (!stopped && find_attempt (nano::bootstrap_mode::legacy) == nullptr)
	{
		node.stats->inc (nano::stat::type::bootstrap, frontiers_age_a == std::numeric_limits<uint32_t>::max () ? nano::stat::detail::initiate : nano::stat::detail::initiate_legacy_age, nano::stat::dir::out);
		auto legacy_attempt (std::make_shared<nano::bootstrap_attempt_legacy> (node.shared (), attempts.create_incremental_id (), id_a, frontiers_age_a, start_account_a));
		attempts_list.push_back (legacy_attempt);
		attempts.add (legacy_attempt);
		lock.unlock ();
		condition.notify_all ();
	}
}

void nano::bootstrap_initiator::bootstrap (nano::endpoint const & endpoint_a, bool add_to_peers, std::string id_a)
{
	if (add_to_peers)
	{
		if (!node.flags.disable_udp ())
		{
			node.network->udp_channels.insert (nano::transport::map_endpoint_to_v6 (endpoint_a), node.network_params.network.protocol_version);
		}
		else if (!node.flags.disable_tcp_realtime ())
		{
			node.network->merge_peer (nano::transport::map_endpoint_to_v6 (endpoint_a));
		}
	}
	if (!stopped)
	{
		stop_attempts ();
		node.stats->inc (nano::stat::type::bootstrap, nano::stat::detail::initiate, nano::stat::dir::out);
		nano::lock_guard<nano::mutex> lock{ mutex };
		auto legacy_attempt (std::make_shared<nano::bootstrap_attempt_legacy> (node.shared (), attempts.create_incremental_id (), id_a, std::numeric_limits<uint32_t>::max (), 0));
		attempts_list.push_back (legacy_attempt);
		attempts.add (legacy_attempt);
		if (!node.network->excluded_peers.check (nano::transport::map_endpoint_to_tcp (endpoint_a)))
		{
			connections->add_connection (endpoint_a);
		}
	}
	condition.notify_all ();
}

bool nano::bootstrap_initiator::bootstrap_lazy (nano::hash_or_account const & hash_or_account_a, bool force, std::string id_a)
{
	bool key_inserted (false);
	auto lazy_attempt (current_lazy_attempt ());
	if (lazy_attempt == nullptr || force)
	{
		if (force)
		{
			stop_attempts ();
		}
		node.stats->inc (nano::stat::type::bootstrap, nano::stat::detail::initiate_lazy, nano::stat::dir::out);
		nano::lock_guard<nano::mutex> lock{ mutex };
		if (!stopped && find_attempt (nano::bootstrap_mode::lazy) == nullptr)
		{
			lazy_attempt = std::make_shared<nano::bootstrap_attempt_lazy> (node.shared (), attempts.create_incremental_id (), id_a.empty () ? hash_or_account_a.to_string () : id_a);
			attempts_list.push_back (lazy_attempt);
			attempts.add (lazy_attempt);
			key_inserted = lazy_attempt->lazy_start (hash_or_account_a);
		}
	}
	else
	{
		key_inserted = lazy_attempt->lazy_start (hash_or_account_a);
	}
	condition.notify_all ();
	return key_inserted;
}

void nano::bootstrap_initiator::bootstrap_wallet (std::deque<nano::account> & accounts_a)
{
	debug_assert (!accounts_a.empty ());
	auto wallet_attempt (current_wallet_attempt ());
	node.stats->inc (nano::stat::type::bootstrap, nano::stat::detail::initiate_wallet_lazy, nano::stat::dir::out);
	if (wallet_attempt == nullptr)
	{
		nano::lock_guard<nano::mutex> lock{ mutex };
		std::string id (!accounts_a.empty () ? accounts_a[0].to_account () : "");
		wallet_attempt = std::make_shared<nano::bootstrap_attempt_wallet> (node.shared (), attempts.create_incremental_id (), id);
		attempts_list.push_back (wallet_attempt);
		attempts.add (wallet_attempt);
		wallet_attempt->wallet_start (accounts_a);
	}
	else
	{
		wallet_attempt->wallet_start (accounts_a);
	}
	condition.notify_all ();
}

void nano::bootstrap_initiator::run_bootstrap ()
{
	nano::unique_lock<nano::mutex> lock{ mutex };
	while (!stopped)
	{
		if (has_new_attempts ())
		{
			auto attempt (new_attempt ());
			lock.unlock ();
			if (attempt != nullptr)
			{
				attempt->run ();
				remove_attempt (attempt);
			}
			lock.lock ();
		}
		else
		{
			condition.wait (lock);
		}
	}
}

void nano::bootstrap_initiator::lazy_requeue (nano::block_hash const & hash_a, nano::block_hash const & previous_a)
{
	auto lazy_attempt (current_lazy_attempt ());
	if (lazy_attempt != nullptr)
	{
		lazy_attempt->lazy_requeue (hash_a, previous_a);
	}
}

bool nano::bootstrap_initiator::in_progress ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	return !attempts_list.empty ();
}

void nano::bootstrap_initiator::block_processed (nano::transaction const & tx, nano::process_return const & result, nano::block const & block)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	for (auto & i : attempts_list)
	{
		i->block_processed (tx, result, block);
	}
}

std::shared_ptr<nano::bootstrap_attempt> nano::bootstrap_initiator::find_attempt (nano::bootstrap_mode mode_a)
{
	for (auto & i : attempts_list)
	{
		if (i->get_mode () == mode_a)
		{
			return i;
		}
	}
	return nullptr;
}

void nano::bootstrap_initiator::remove_attempt (std::shared_ptr<nano::bootstrap_attempt> attempt_a)
{
	nano::unique_lock<nano::mutex> lock{ mutex };
	auto attempt (std::find (attempts_list.begin (), attempts_list.end (), attempt_a));
	if (attempt != attempts_list.end ())
	{
		auto attempt_ptr (*attempt);
		attempts.remove (attempt_ptr->get_incremental_id ());
		attempts_list.erase (attempt);
		debug_assert (attempts.size () == attempts_list.size ());
		lock.unlock ();
		attempt_ptr->stop ();
	}
	else
	{
		lock.unlock ();
	}
	condition.notify_all ();
}

std::shared_ptr<nano::bootstrap_attempt> nano::bootstrap_initiator::new_attempt ()
{
	for (auto & i : attempts_list)
	{
		if (!i->set_started ())
		{
			return i;
		}
	}
	return nullptr;
}

bool nano::bootstrap_initiator::has_new_attempts ()
{
	for (auto & i : attempts_list)
	{
		if (!i->get_started ())
		{
			return true;
		}
	}
	return false;
}

std::shared_ptr<nano::bootstrap_attempt> nano::bootstrap_initiator::current_attempt ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	return find_attempt (nano::bootstrap_mode::legacy);
}

std::shared_ptr<nano::bootstrap_attempt_lazy> nano::bootstrap_initiator::current_lazy_attempt ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	return std::dynamic_pointer_cast<nano::bootstrap_attempt_lazy> (find_attempt (nano::bootstrap_mode::lazy));
}

std::shared_ptr<nano::bootstrap_attempt_wallet> nano::bootstrap_initiator::current_wallet_attempt ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	return std::dynamic_pointer_cast<nano::bootstrap_attempt_wallet> (find_attempt (nano::bootstrap_mode::wallet_lazy));
}

void nano::bootstrap_initiator::stop_attempts ()
{
	nano::unique_lock<nano::mutex> lock{ mutex };
	std::vector<std::shared_ptr<nano::bootstrap_attempt>> copy_attempts;
	copy_attempts.swap (attempts_list);
	attempts.clear ();
	lock.unlock ();
	for (auto & i : copy_attempts)
	{
		i->stop ();
	}
}

void nano::bootstrap_initiator::clear_pulls (uint64_t bootstrap_id_a)
{
	connections->clear_pulls (bootstrap_id_a);
}

rsnano::BootstrapInitiatorHandle * nano::bootstrap_initiator::get_handle () const
{
	return handle;
}

void nano::bootstrap_initiator::stop ()
{
	if (!stopped.exchange (true))
	{
		stop_attempts ();
		connections->stop ();
		condition.notify_all ();

		for (auto & thread : bootstrap_initiator_threads)
		{
			if (thread.joinable ())
			{
				thread.join ();
			}
		}
	}
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (bootstrap_initiator & bootstrap_initiator, std::string const & name)
{
	auto cache_count = bootstrap_initiator.cache.size ();
	auto sizeof_cache_element = pulls_cache::element_size ();
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "pulls_cache", cache_count, sizeof_cache_element }));
	return composite;
}

nano::pulls_cache::pulls_cache () :
	handle{ rsnano::rsn_pulls_cache_create () }
{
}

nano::pulls_cache::~pulls_cache ()
{
	rsnano::rsn_pulls_cache_destroy (handle);
}

void nano::pulls_cache::add (nano::pull_info const & pull_a)
{
	auto dto{ pull_a.to_dto () };
	rsnano::rsn_pulls_cache_add (handle, &dto);
}

void nano::pulls_cache::update_pull (nano::pull_info & pull_a)
{
	auto dto{ pull_a.to_dto () };
	rsnano::rsn_pulls_cache_update_pull (handle, &dto);
	pull_a.load_dto (dto);
}

void nano::pulls_cache::remove (nano::pull_info const & pull_a)
{
	auto dto{ pull_a.to_dto () };
	rsnano::rsn_pulls_cache_remove (handle, &dto);
}

size_t nano::pulls_cache::size ()
{
	return rsnano::rsn_pulls_cache_size (handle);
}
size_t nano::pulls_cache::element_size ()
{
	return rsnano::rsn_pulls_cache_element_size ();
}

nano::bootstrap_attempts::bootstrap_attempts () :
	handle{ rsnano::rsn_bootstrap_attempts_create () }
{
}

nano::bootstrap_attempts::~bootstrap_attempts () noexcept
{
	rsnano::rsn_bootstrap_attempts_destroy (handle);
}

void nano::bootstrap_attempts::add (std::shared_ptr<nano::bootstrap_attempt> attempt_a)
{
	nano::lock_guard<nano::mutex> lock{ bootstrap_attempts_mutex };
	attempts.emplace (attempt_a->get_incremental_id (), attempt_a);
	//	rsnano::rsn_bootstrap_attempts_add (handle, attempt_a->handle);
}

void nano::bootstrap_attempts::remove (uint64_t incremental_id_a)
{
	nano::lock_guard<nano::mutex> lock{ bootstrap_attempts_mutex };
	attempts.erase (incremental_id_a);
	//	rsnano::rsn_bootstrap_attempts_remove (handle, incremental_id_a);
}

void nano::bootstrap_attempts::clear ()
{
	nano::lock_guard<nano::mutex> lock{ bootstrap_attempts_mutex };
	attempts.clear ();
	//	rsnano::rsn_bootstrap_attempts_clear (handle);
}

std::shared_ptr<nano::bootstrap_attempt> nano::bootstrap_attempts::find (uint64_t incremental_id_a)
{
	nano::lock_guard<nano::mutex> lock{ bootstrap_attempts_mutex };
	auto find_attempt (attempts.find (incremental_id_a));
	if (find_attempt != attempts.end ())
	{
		return find_attempt->second;
	}
	else
	{
		return nullptr;
	}
	//	auto attempt_handle = rsnano::rsn_bootstrap_attempts_find (handle, incremental_id_a);
	//	if (attempt_handle){
	//		return rsnano::dto_to_bootstrap_attempt (attempt_handle);
	//	}
	//	else
	//	{
	//		return nullptr;
	//	}
}
std::size_t nano::bootstrap_attempts::size ()
{
	nano::lock_guard<nano::mutex> lock{ bootstrap_attempts_mutex };
	return attempts.size ();
	//return rsnano::rsn_bootstrap_attempts_size (handle);
}
uint64_t nano::bootstrap_attempts::create_incremental_id ()
{
	return incremental++;
	//	return rsnano::rsn_bootstrap_attempts_get_incremental_id (handle);
}
uint64_t nano::bootstrap_attempts::total_attempts () const
{
	return incremental;
	//	return rsnano::rsn_bootstrap_attempts_total_attempts (handle);
}
std::map<uint64_t, std::shared_ptr<nano::bootstrap_attempt>> nano::bootstrap_attempts::get_attempts ()
{
	nano::lock_guard<nano::mutex> lock (bootstrap_attempts_mutex);
	return attempts;
	//	std::vector<rsnano::BootstrapAttemptResultDto> dtos;
	//	auto max_size = rsnano::rsn_bootstrap_attempts_size (handle);
	//	dtos.resize (max_size);
	//	auto actual_size = rsnano::rsn_bootstrap_attempts_attempts (handle, dtos.data(), max_size);
	//	std::map<uint64_t, std::shared_ptr<nano::bootstrap_attempt>> result;
	//	for (int i = 0; i < actual_size; ++i)
	//	{
	//		result.insert(dtos[i].id, rsnano::dto_to_bootstrap_attempt (dtos[i].attempt));
	//	}
	//	return result;
}
