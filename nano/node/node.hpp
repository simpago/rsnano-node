#pragma once

#include "nano/secure/common.hpp"

#include <nano/lib/config.hpp>
#include <nano/lib/logging.hpp>
#include <nano/lib/stats.hpp>
#include <nano/lib/thread_pool.hpp>
#include <nano/lib/work.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/backlog_population.hpp>
#include <nano/node/bandwidth_limiter.hpp>
#include <nano/node/blockprocessor.hpp>
#include <nano/node/bootstrap/bootstrap.hpp>
#include <nano/node/bootstrap/bootstrap_server.hpp>
#include <nano/node/confirming_set.hpp>
#include <nano/node/distributed_work_factory.hpp>
#include <nano/node/election.hpp>
#include <nano/node/local_vote_history.hpp>
#include <nano/node/network.hpp>
#include <nano/node/node_observers.hpp>
#include <nano/node/nodeconfig.hpp>
#include <nano/node/rep_tiers.hpp>
#include <nano/node/repcrawler.hpp>
#include <nano/node/request_aggregator.hpp>
#include <nano/node/telemetry.hpp>
#include <nano/node/unchecked_map.hpp>
#include <nano/node/vote_cache.hpp>
#include <nano/node/vote_processor.hpp>
#include <nano/node/wallet.hpp>
#include <nano/node/websocket.hpp>
#include <nano/secure/utility.hpp>

#include <boost/program_options.hpp>
#include <boost/thread/latch.hpp>

#include <memory>
#include <optional>

namespace nano
{
class node;
class work_pool;

namespace scheduler
{
	class component;
}
namespace transport
{
	class tcp_listener;
}

class ConfirmationQuorum
{
public:
	nano::amount quorum_delta;
	uint8_t online_weight_quorum_percent;
	nano::amount online_weight_minimum;
	nano::amount online_weight;
	nano::amount trended_weight;
	nano::amount peers_weight;
	nano::amount minimum_principal_weight;
};

class node final : public std::enable_shared_from_this<nano::node>
{
public:
	std::shared_ptr<nano::node_observers> observers; // TODO
	rsnano::NodeHandle * handle;

	node (rsnano::async_runtime & async_rt_a, uint16_t peering_port, std::filesystem::path const & application_path, nano::work_pool &, nano::node_flags = nano::node_flags (), unsigned seq = 0);
	node (rsnano::async_runtime & async_rt_a, std::filesystem::path const & application_path, nano::node_config const &, nano::work_pool &, nano::node_flags = nano::node_flags (), unsigned seq = 0);
	node (node const &) = delete;
	~node ();

public:
	bool copy_with_compaction (std::filesystem::path const &);
	void keepalive (std::string const &, uint16_t);
	void start ();
	void stop ();
	bool is_stopped () const;
	std::shared_ptr<nano::node> shared ();
	int store_version ();
	void process_confirmed (nano::election_status const &, uint64_t = 0);
	void process_active (std::shared_ptr<nano::block> const &);
	std::optional<nano::block_status> process_local (std::shared_ptr<nano::block> const &);
	void process_local_async (std::shared_ptr<nano::block> const &);
	std::shared_ptr<nano::block> block (nano::block_hash const &);
	bool block_or_pruned_exists (nano::block_hash const &) const;
	std::pair<nano::uint128_t, nano::uint128_t> balance_pending (nano::account const &, bool only_confirmed);
	nano::uint128_t weight (nano::account const &);
	nano::uint128_t minimum_principal_weight ();
	void bootstrap_wallet ();
	void ledger_pruning (uint64_t const, bool);
	int price (nano::uint128_t const &, int);
	// The default difficulty updates to base only when the first epoch_2 block is processed
	uint64_t default_difficulty (nano::work_version const) const;
	uint64_t default_receive_difficulty (nano::work_version const) const;
	uint64_t max_work_generate_difficulty (nano::work_version const) const;
	bool local_work_generation_enabled () const;
	bool work_generation_enabled () const;
	std::optional<uint64_t> work_generate_blocking (nano::block &, uint64_t);
	std::optional<uint64_t> work_generate_blocking (nano::work_version const, nano::root const &, uint64_t, std::optional<nano::account> const & = std::nullopt);
	void work_generate (nano::work_version const, nano::root const &, uint64_t, std::function<void (std::optional<uint64_t>)>, std::optional<nano::account> const & = std::nullopt, bool const = false);
	void start_election (std::shared_ptr<nano::block> const & block);
	bool block_confirmed (nano::block_hash const &);

	// This function may spuriously return false after returning true until the database transaction is refreshed
	bool block_confirmed_or_being_confirmed (nano::store::transaction const &, nano::block_hash const &);
	bool block_confirmed_or_being_confirmed (nano::block_hash const &);

	nano::vote_code vote (nano::vote const & vote, nano::block_hash hash = nano::block_hash (0));
	bool election_active (nano::block_hash const & hash) const;
	bool init_error () const;
	uint64_t get_confirmation_height (store::transaction const &, nano::account &);
	nano::account get_node_id () const;
	nano::telemetry_data local_telemetry () const;
	void connect (nano::endpoint const &);
	void enqueue_vote_request (nano::root const & root, nano::block_hash const & hash);
	nano::amount get_rep_weight (nano::account const & account);
	std::unordered_map<nano::account, nano::uint128_t> get_rep_weights () const;
	nano::ConfirmationQuorum quorum () const;

public:
	nano::keypair node_id;
	rsnano::async_runtime & async_rt;
	boost::asio::io_context & io_ctx;
	std::shared_ptr<nano::node_config> config;
	nano::network_params network_params;
	std::shared_ptr<nano::logger> logger;
	std::shared_ptr<nano::stats> stats;
	std::shared_ptr<nano::thread_pool> workers;
	std::shared_ptr<nano::thread_pool> bootstrap_workers;
	nano::node_flags flags;
	nano::work_pool & work;
	nano::distributed_work_factory distributed_work;
	nano::store::lmdb::component store;
	nano::unchecked_map unchecked;
	nano::ledger ledger;
	nano::outbound_bandwidth_limiter outbound_limiter;
	std::shared_ptr<nano::network> network;
	std::shared_ptr<nano::telemetry> telemetry;
	nano::bootstrap_server bootstrap_server;
	std::filesystem::path application_path;
	nano::representative_register representative_register;
	nano::rep_tiers rep_tiers;
	nano::vote_processor_queue vote_processor_queue;
	nano::local_vote_history history;
	nano::confirming_set confirming_set;
	nano::vote_cache vote_cache;
	nano::block_processor block_processor;
	nano::wallets wallets;
	nano::active_elections active;
	nano::vote_processor vote_processor;
	nano::websocket_server websocket;
	nano::bootstrap_initiator bootstrap_initiator;
	nano::rep_crawler rep_crawler;
	std::shared_ptr<nano::transport::tcp_listener> tcp_listener;

private: // Placed here to maintain initialization order
	std::unique_ptr<nano::scheduler::component> scheduler_impl;

public:
	nano::scheduler::component & scheduler;
	nano::request_aggregator aggregator;
	nano::backlog_population backlog;

	std::chrono::steady_clock::time_point const startup_time;
	// For tests only
	unsigned node_seq;
	// For tests only
	std::optional<uint64_t> work_generate_blocking (nano::block &);
	// For tests only
	std::optional<uint64_t> work_generate_blocking (nano::root const &, uint64_t);
	// For tests only
	std::optional<uint64_t> work_generate_blocking (nano::root const &);

public: // Testing convenience functions
	/**
		Creates a new write transaction and inserts `block' and returns result
		Transaction is comitted before function return
	 */
	[[nodiscard]] nano::block_status process (std::shared_ptr<nano::block> block);
	[[nodiscard]] nano::block_status process (store::write_transaction const &, std::shared_ptr<nano::block> block);
	nano::block_hash latest (nano::account const &);
	nano::uint128_t balance (nano::account const &);
	std::vector<nano::account> list_online_reps ();
	void set_online_weight (nano::uint128_t online_a);

private:
	static std::string make_logger_identifier (nano::keypair const & node_id);
};

std::unique_ptr<container_info_component> collect_container_info (node & node, std::string const & name);

nano::node_flags const & inactive_node_flag_defaults ();

}
