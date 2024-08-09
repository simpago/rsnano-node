#include "nano/lib/numbers.hpp"

#include <nano/lib/blocks.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/stream.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/tomlconfig.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/common.hpp>
#include <nano/node/daemonconfig.hpp>
#include <nano/node/election_status.hpp>
#include <nano/node/local_vote_history.hpp>
#include <nano/node/make_store.hpp>
#include <nano/node/node.hpp>
#include <nano/node/scheduler/component.hpp>
#include <nano/node/scheduler/hinted.hpp>
#include <nano/node/scheduler/manual.hpp>
#include <nano/node/scheduler/optimistic.hpp>
#include <nano/node/scheduler/priority.hpp>
#include <nano/node/telemetry.hpp>
#include <nano/node/transport/tcp_listener.hpp>
#include <nano/node/websocket.hpp>
#include <nano/secure/common.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/store/component.hpp>
#include <nano/store/write_queue.hpp>

#include <boost/property_tree/json_parser.hpp>

#include <memory>

/*
 * node
 */

void nano::node::keepalive (std::string const & address_a, uint16_t port_a)
{
	rsnano::rsn_rep_crawler_keepalive (rep_crawler.handle, address_a.c_str (), port_a);
}

namespace
{
nano::keypair get_node_id_key_pair (rsnano::NodeHandle const * handle)
{
	nano::raw_key prv;
	rsnano::rsn_node_node_id (handle, prv.bytes.data ());
	return std::move (nano::keypair{ std::move (prv) });
}

std::shared_ptr<nano::node_config> get_node_config (rsnano::NodeHandle const * handle)
{
	rsnano::NodeConfigDto dto;
	rsn_node_config (handle, &dto);
	auto config = std::make_shared<nano::node_config> ();
	config->load_dto (dto);
	return config;
}

namespace
{
	void delete_observers_context (void * context)
	{
		auto observers = static_cast<std::weak_ptr<nano::node_observers> *> (context);
		delete observers;
	}

	void call_election_ended (void * context, rsnano::ElectionStatusHandle * status_handle,
	rsnano::VoteWithWeightInfoVecHandle * votes_handle, uint8_t const * account_bytes,
	uint8_t const * amount_bytes, bool is_state_send, bool is_state_epoch)
	{
		auto observers = static_cast<std::weak_ptr<nano::node_observers> *> (context);
		auto obs = observers->lock ();
		if (!obs)
		{
			return;
		}

		nano::election_status status{ status_handle };

		std::vector<nano::vote_with_weight_info> votes;
		auto len = rsnano::rsn_vote_with_weight_info_vec_len (votes_handle);
		for (auto i = 0; i < len; ++i)
		{
			rsnano::VoteWithWeightInfoDto dto;
			rsnano::rsn_vote_with_weight_info_vec_get (votes_handle, i, &dto);
			votes.emplace_back (dto);
		}
		rsnano::rsn_vote_with_weight_info_vec_destroy (votes_handle);

		auto account{ nano::account::from_bytes (account_bytes) };
		auto amount{ nano::amount::from_bytes (amount_bytes) };

		obs->blocks.notify (status, votes, account, amount.number (), is_state_send, is_state_epoch);
	}

	void call_account_balance_changed (void * context, uint8_t const * account, bool is_pending)
	{
		auto observers = static_cast<std::weak_ptr<nano::node_observers> *> (context);
		auto obs = observers->lock ();
		if (!obs)
		{
			return;
		}
		obs->account_balance.notify (nano::account::from_bytes (account), is_pending);
	}

	void on_vote_processed (void * context, rsnano::VoteHandle * vote_handle, uint8_t source, uint8_t code)
	{
		auto observers = static_cast<std::weak_ptr<nano::node_observers> *> (context);
		auto obs = observers->lock ();
		if (!obs)
		{
			return;
		}
		auto vote = std::make_shared<nano::vote> (vote_handle);
		obs->vote.notify (vote, static_cast<nano::vote_source> (source), static_cast<nano::vote_code> (code));
	}

}

rsnano::NodeHandle * create_node_handle (
rsnano::async_runtime & async_rt_a,
std::filesystem::path const & application_path_a,
nano::node_config const & config_a,
nano::work_pool & work_a,
nano::node_flags flags_a,
unsigned seq,
std::shared_ptr<nano::node_observers> & observers)
{
	auto config_dto{ config_a.to_dto () };
	auto params_dto{ config_a.network_params.to_dto () };
	auto observers_context = new std::weak_ptr<nano::node_observers> (observers);
	return rsnano::rsn_node_create (application_path_a.c_str (), async_rt_a.handle, &config_dto, &params_dto,
	flags_a.handle, work_a.handle, observers_context, delete_observers_context,
	call_election_ended, call_account_balance_changed, on_vote_processed);
}
}

nano::node::node (rsnano::async_runtime & async_rt, uint16_t peering_port_a, std::filesystem::path const & application_path_a, nano::work_pool & work_a, nano::node_flags flags_a, unsigned seq) :
	node (async_rt, application_path_a, nano::node_config (peering_port_a), work_a, flags_a, seq)
{
}

nano::node::node (rsnano::async_runtime & async_rt_a, std::filesystem::path const & application_path_a, nano::node_config const & config_a, nano::work_pool & work_a, nano::node_flags flags_a, unsigned seq) :
	observers{ std::make_shared<nano::node_observers> () },
	handle{ create_node_handle (async_rt_a, application_path_a, config_a, work_a, flags_a, seq, observers) },
	node_id{ get_node_id_key_pair (handle) },
	async_rt{ async_rt_a },
	io_ctx (async_rt_a.io_ctx),
	config{ get_node_config (handle) },
	network_params{ config_a.network_params },
	logger{ std::make_shared<nano::logger> (make_logger_identifier (node_id)) },
	stats{ std::make_shared<nano::stats> (rsnano::rsn_node_stats (handle)) },
	workers{ std::make_shared<nano::thread_pool> (rsnano::rsn_node_workers (handle)) },
	bootstrap_workers{ std::make_shared<nano::thread_pool> (rsnano::rsn_node_bootstrap_workers (handle)) },
	flags (flags_a),
	work (work_a),
	distributed_work (rsnano::rsn_node_distributed_work (handle)),
	store (rsnano::rsn_node_store (handle)),
	unchecked{ rsn_node_unchecked (handle) },
	ledger (rsnano::rsn_node_ledger (handle), store, network_params.ledger),
	outbound_limiter{ rsnano::rsn_node_outbound_bandwidth_limiter (handle) },
	// empty `config.peering_port` means the user made no port choice at all;
	// otherwise, any value is considered, with `0` having the special meaning of 'let the OS pick a port instead'
	network{ std::make_shared<nano::network> (*this, config_a.peering_port.value_or (0), rsnano::rsn_node_syn_cookies (handle), rsnano::rsn_node_tcp_channels (handle), rsnano::rsn_node_network_filter (handle)) },
	telemetry (std::make_shared<nano::telemetry> (rsnano::rsn_node_telemetry (handle))),
	bootstrap_initiator (rsnano::rsn_node_bootstrap_initiator (handle)),
	bootstrap_server{ rsnano::rsn_node_bootstrap_server (handle) },
	// BEWARE: `bootstrap` takes `network.port` instead of `config.peering_port` because when the user doesn't specify
	//         a peering port and wants the OS to pick one, the picking happens when `network` gets initialized
	//         (if UDP is active, otherwise it happens when `bootstrap` gets initialized), so then for TCP traffic
	//         we want to tell `bootstrap` to use the already picked port instead of itself picking a different one.
	//         Thus, be very careful if you change the order: if `bootstrap` gets constructed before `network`,
	//         the latter would inherit the port from the former (if TCP is active, otherwise `network` picks first)
	//
	tcp_listener{ std::make_shared<nano::transport::tcp_listener> (rsnano::rsn_node_tcp_listener (handle)) },
	application_path (application_path_a),
	representative_register (rsnano::rsn_node_representative_register (handle)),
	rep_crawler (rsnano::rsn_node_rep_crawler (handle), *this),
	rep_tiers{ rsnano::rsn_node_rep_tiers (handle) },
	vote_processor_queue{
		rsnano::rsn_node_vote_processor_queue (handle)
	},
	vote_processor (rsnano::rsn_node_vote_processor (handle)),
	block_processor (rsnano::rsn_node_block_processor (handle)),
	history{ rsnano::rsn_node_history (handle) },
	confirming_set (rsnano::rsn_node_confirming_set (handle)),
	vote_cache{ rsnano::rsn_node_vote_cache (handle) },
	wallets{ rsnano::rsn_node_wallets (handle) },
	active (*this, rsnano::rsn_node_active (handle)),
	scheduler_impl{ std::make_unique<nano::scheduler::component> (handle) },
	scheduler{ *scheduler_impl },
	aggregator (rsnano::rsn_node_request_aggregator (handle)),
	backlog{ rsnano::rsn_node_backlog_population (handle) },
	websocket{ rsnano::rsn_node_websocket (handle) },
	startup_time (std::chrono::steady_clock::now ()),
	node_seq (seq)
{
}

nano::node::~node ()
{
	logger->debug (nano::log::type::node, "Destructing node...");
	stop ();
	rsnano::rsn_node_destroy (handle);
}

namespace
{
void call_post_callback (void * callback_handle)
{
	auto callback = static_cast<std::function<void ()> *> (callback_handle);
	(*callback) ();
}

void delete_post_callback (void * callback_handle)
{
	auto callback = static_cast<std::function<void ()> *> (callback_handle);
	delete callback;
}
}

bool nano::node::copy_with_compaction (std::filesystem::path const & destination)
{
	return store.copy_db (destination);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (node & node, std::string const & name)
{
	return std::make_unique<container_info_composite> (rsnano::rsn_node_collect_container_info (node.handle, name.c_str ()));
}

void nano::node::process_active (std::shared_ptr<nano::block> const & incoming)
{
	block_processor.process_active (incoming);
}

[[nodiscard]] nano::block_status nano::node::process (store::write_transaction const & transaction, std::shared_ptr<nano::block> block)
{
	return ledger.process (transaction, block);
}

nano::block_status nano::node::process (std::shared_ptr<nano::block> block)
{
	auto const transaction = store.tx_begin_write ({ tables::accounts, tables::blocks, tables::pending, tables::rep_weights });
	return process (*transaction, block);
}

std::optional<nano::block_status> nano::node::process_local (std::shared_ptr<nano::block> const & block_a)
{
	return block_processor.add_blocking (block_a, nano::block_source::local);
}

void nano::node::process_local_async (std::shared_ptr<nano::block> const & block_a)
{
	block_processor.add (block_a, nano::block_source::local);
}

void nano::node::start ()
{
	rsnano::rsn_node_start (handle);
}

void nano::node::stop ()
{
	rsnano::rsn_node_stop (handle);
}

bool nano::node::is_stopped () const
{
	return rsnano::rsn_node_is_stopped (handle);
}

nano::block_hash nano::node::latest (nano::account const & account_a)
{
	auto const transaction (store.tx_begin_read ());
	return ledger.any ().account_head (*transaction, account_a);
}

nano::uint128_t nano::node::balance (nano::account const & account_a)
{
	auto const transaction (store.tx_begin_read ());
	return ledger.any ().account_balance (*transaction, account_a).value_or (0).number ();
}

std::shared_ptr<nano::block> nano::node::block (nano::block_hash const & hash_a)
{
	auto const transaction (store.tx_begin_read ());
	return ledger.any ().block_get (*transaction, hash_a);
}

bool nano::node::block_or_pruned_exists (nano::block_hash const & hash_a) const
{
	return ledger.any ().block_exists_or_pruned (*ledger.store.tx_begin_read (), hash_a);
}

std::pair<nano::uint128_t, nano::uint128_t> nano::node::balance_pending (nano::account const & account_a, bool only_confirmed_a)
{
	std::pair<nano::uint128_t, nano::uint128_t> result;
	auto const transaction (store.tx_begin_read ());
	result.first = only_confirmed_a ? ledger.confirmed ().account_balance (*transaction, account_a).value_or (0).number () : ledger.any ().account_balance (*transaction, account_a).value_or (0).number ();
	result.second = ledger.account_receivable (*transaction, account_a, only_confirmed_a);
	return result;
}

nano::uint128_t nano::node::weight (nano::account const & account_a)
{
	auto txn{ ledger.store.tx_begin_read () };
	return ledger.weight_exact (*txn, account_a);
}

nano::uint128_t nano::node::minimum_principal_weight ()
{
	return quorum ().minimum_principal_weight.number ();
}

void nano::node::bootstrap_wallet ()
{
	rsnano::rsn_node_bootstrap_wallet (handle);
}

void nano::node::ledger_pruning (uint64_t const batch_size_a, bool bootstrap_weight_reached_a)
{
	rsnano::rsn_node_ledger_pruning (handle, batch_size_a, bootstrap_weight_reached_a);
}

uint64_t nano::node::default_difficulty (nano::work_version const version_a) const
{
	uint64_t result{ std::numeric_limits<uint64_t>::max () };
	switch (version_a)
	{
		case nano::work_version::work_1:
			result = network_params.work.threshold_base (version_a);
			break;
		default:
			debug_assert (false && "Invalid version specified to default_difficulty");
	}
	return result;
}

uint64_t nano::node::default_receive_difficulty (nano::work_version const version_a) const
{
	uint64_t result{ std::numeric_limits<uint64_t>::max () };
	switch (version_a)
	{
		case nano::work_version::work_1:
			result = network_params.work.get_epoch_2_receive ();
			break;
		default:
			debug_assert (false && "Invalid version specified to default_receive_difficulty");
	}
	return result;
}

uint64_t nano::node::max_work_generate_difficulty (nano::work_version const version_a) const
{
	return nano::difficulty::from_multiplier (config->max_work_generate_multiplier, default_difficulty (version_a));
}

bool nano::node::local_work_generation_enabled () const
{
	return work.work_generation_enabled ();
}

bool nano::node::work_generation_enabled () const
{
	return distributed_work.work_generation_enabled ();
}

std::optional<uint64_t> nano::node::work_generate_blocking (nano::block & block_a, uint64_t difficulty_a)
{
	return distributed_work.make_blocking (block_a, difficulty_a);
}

void nano::node::work_generate (nano::work_version const version_a, nano::root const & root_a, uint64_t difficulty_a, std::function<void (std::optional<uint64_t>)> callback_a, std::optional<nano::account> const & account_a, bool secondary_work_peers_a)
{
	distributed_work.make (version_a, root_a, difficulty_a, callback_a, account_a, secondary_work_peers_a);
}

std::optional<uint64_t> nano::node::work_generate_blocking (nano::work_version const version_a, nano::root const & root_a, uint64_t difficulty_a, std::optional<nano::account> const & account_a)
{
	return distributed_work.make_blocking (version_a, root_a, difficulty_a, account_a);
}

std::optional<uint64_t> nano::node::work_generate_blocking (nano::block & block_a)
{
	debug_assert (network_params.network.is_dev_network ());
	return work_generate_blocking (block_a, default_difficulty (nano::work_version::work_1));
}

std::optional<uint64_t> nano::node::work_generate_blocking (nano::root const & root_a)
{
	debug_assert (network_params.network.is_dev_network ());
	return work_generate_blocking (root_a, default_difficulty (nano::work_version::work_1));
}

std::optional<uint64_t> nano::node::work_generate_blocking (nano::root const & root_a, uint64_t difficulty_a)
{
	debug_assert (network_params.network.is_dev_network ());
	return work_generate_blocking (nano::work_version::work_1, root_a, difficulty_a);
}

void nano::node::start_election (std::shared_ptr<nano::block> const & block)
{
	scheduler.manual.push (block);
}

bool nano::node::block_confirmed (nano::block_hash const & hash_a)
{
	auto transaction (store.tx_begin_read ());
	return ledger.confirmed ().block_exists_or_pruned (*transaction, hash_a);
}

bool nano::node::block_confirmed_or_being_confirmed (nano::store::transaction const & transaction, nano::block_hash const & hash_a)
{
	return confirming_set.exists (hash_a) || ledger.confirmed ().block_exists_or_pruned (transaction, hash_a);
}

bool nano::node::block_confirmed_or_being_confirmed (nano::block_hash const & hash_a)
{
	return block_confirmed_or_being_confirmed (*store.tx_begin_read (), hash_a);
}

nano::vote_code nano::node::vote (nano::vote const & vote, nano::block_hash hash)
{
	return static_cast<nano::vote_code> (rsnano::rsn_node_vote (handle, vote.get_handle (), hash.bytes.data ()));
}

bool nano::node::election_active (nano::block_hash const & hash) const
{
	return rsnano::rsn_node_election_active (handle, hash.bytes.data ());
}

void nano::node::process_confirmed (nano::election_status const & status_a, uint64_t iteration_a)
{
	active.process_confirmed (status_a, iteration_a);
}

std::shared_ptr<nano::node> nano::node::shared ()
{
	return shared_from_this ();
}

int nano::node::store_version ()
{
	auto transaction (store.tx_begin_read ());
	return store.version ().get (*transaction);
}

bool nano::node::init_error () const
{
	return store.init_error ();
}

/** Convenience function to easily return the confirmation height of an account. */
uint64_t nano::node::get_confirmation_height (store::transaction const & transaction_a, nano::account & account_a)
{
	nano::confirmation_height_info info;
	store.confirmation_height ().get (transaction_a, account_a, info);
	return info.height ();
}

nano::account nano::node::get_node_id () const
{
	return node_id.pub;
};

nano::telemetry_data nano::node::local_telemetry () const
{
	return telemetry->local_telemetry ();
}

void nano::node::connect (nano::endpoint const & endpoint)
{
	auto dto{ rsnano::udp_endpoint_to_dto (endpoint) };
	rsnano::rsn_node_connect (handle, &dto);
}

void nano::node::enqueue_vote_request (nano::root const & root, nano::block_hash const & hash)
{
	rsnano::rsn_node_enqueue_vote_request (handle, root.bytes.data (), hash.bytes.data ());
}

nano::amount nano::node::get_rep_weight (nano::account const & account)
{
	nano::amount weight;
	rsnano::rsn_node_get_rep_weight (handle, account.bytes.data (), weight.bytes.data ());
	return weight;
}

std::unordered_map<nano::account, nano::uint128_t> nano::node::get_rep_weights () const
{
	auto result_handle = rsnano::rsn_node_get_rep_weights (handle);
	std::unordered_map<nano::account, nano::uint128_t> result;
	auto len = rsnano::rsn_rep_weights_vec_len (result_handle);
	for (auto i = 0; i < len; ++i)
	{
		nano::account rep;
		nano::amount weight;
		rsnano::rsn_rep_weights_vec_get (result_handle, i, rep.bytes.data (), weight.bytes.data ());
		result.insert ({ rep, weight.number () });
	}
	rsnano::rsn_rep_weights_vec_destroy (result_handle);
	return result;
}

nano::ConfirmationQuorum nano::node::quorum () const
{
	rsnano::ConfirmationQuorumDto dto;
	rsnano::rsn_node_confirmation_quorum (handle, &dto);
	nano::ConfirmationQuorum result;
	result.quorum_delta = nano::amount::from_bytes (dto.quorum_delta);
	result.online_weight_quorum_percent = dto.online_weight_quorum_percent;
	result.online_weight_minimum = nano::amount::from_bytes (dto.online_weight_minimum);
	result.online_weight = nano::amount::from_bytes (dto.online_weight);
	result.trended_weight = nano::amount::from_bytes (dto.trended_weight);
	result.peers_weight = nano::amount::from_bytes (dto.peers_weight);
	result.minimum_principal_weight = nano::amount::from_bytes (dto.minimum_principal_weight);
	return result;
}

std::vector<nano::account> nano::node::list_online_reps ()
{
	rsnano::U256ArrayDto dto;
	rsnano::rsn_node_list_online_reps (handle, &dto);
	std::vector<nano::account> result;
	result.reserve (dto.count);
	for (int i = 0; i < dto.count; ++i)
	{
		nano::account account;
		std::copy (std::begin (dto.items[i]), std::end (dto.items[i]), std::begin (account.bytes));
		result.push_back (account);
	}
	rsnano::rsn_u256_array_destroy (&dto);
	return result;
}

void nano::node::set_online_weight (nano::uint128_t online_a)
{
	nano::amount online_weight{ online_a };
	rsnano::rsn_node_set_online_weight (handle, online_weight.bytes.data ());
}

std::string nano::node::make_logger_identifier (const nano::keypair & node_id)
{
	// Node identifier consists of first 10 characters of node id
	return node_id.pub.to_node_id ().substr (0, 10);
}
