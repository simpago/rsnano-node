#pragma once

#include <nano/lib/config.hpp>
#include <nano/lib/diagnosticsconfig.hpp>
#include <nano/lib/errors.hpp>
#include <nano/lib/lmdbconfig.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/stats.hpp>
#include <nano/lib/threading.hpp>
#include <nano/node/ipc/ipc_config.hpp>
#include <nano/node/logging.hpp>
#include <nano/node/websocketconfig.hpp>
#include <nano/secure/common.hpp>

#include <chrono>
#include <optional>
#include <vector>

namespace nano
{
class tomlconfig;

enum class frontiers_confirmation_mode : uint8_t
{
	always, // Always confirm frontiers
	automatic, // Always mode if node contains representative with at least 50% of principal weight, less frequest requests if not
	disabled, // Do not confirm frontiers
	invalid
};

/**
 * Node configuration
 */
class node_config
{
public:
	node_config (nano::network_params & network_params = nano::dev::network_params);
	node_config (const std::optional<uint16_t> &, nano::logging const &, nano::network_params & network_params = nano::dev::network_params);
	void load_dto (rsnano::NodeConfigDto & dto);
	rsnano::NodeConfigDto to_dto () const;

	nano::error serialize_toml (nano::tomlconfig &) const;
	nano::error deserialize_toml (nano::tomlconfig &);

	bool upgrade_json (unsigned, nano::jsonconfig &);
	nano::account random_representative () const;
	nano::network_params & network_params;
	std::optional<uint16_t> peering_port{};
	nano::logging logging;
	std::vector<std::pair<std::string, uint16_t>> work_peers;
	std::vector<std::pair<std::string, uint16_t>> secondary_work_peers;
	std::vector<std::string> preconfigured_peers;
	std::vector<nano::account> preconfigured_representatives;
	unsigned bootstrap_fraction_numerator{ 1 };
	nano::amount receive_minimum;
	nano::amount vote_minimum;
	nano::amount rep_crawler_weight_minimum;
	std::chrono::milliseconds vote_generator_delay;
	unsigned vote_generator_threshold;
	nano::amount online_weight_minimum{ 60000 * nano::Gxrb_ratio };
	unsigned election_hint_weight_percent{ 50 };
	unsigned password_fanout{ 1024 };
	unsigned io_threads{ std::max (4u, nano::hardware_concurrency ()) };
	unsigned network_threads{ std::max (4u, nano::hardware_concurrency ()) };
	unsigned work_threads{ std::max (4u, nano::hardware_concurrency ()) };
	/* Use half available threads on the system for signature checking. The calling thread does checks as well, so these are extra worker threads */
	unsigned signature_checker_threads{ std::max (2u, nano::hardware_concurrency () / 2) };
	bool enable_voting{ false };
	unsigned bootstrap_connections{ 4 };
	unsigned bootstrap_connections_max{ 64 };
	unsigned bootstrap_initiator_threads{ 1 };
	unsigned bootstrap_serving_threads{ std::max (2u, nano::hardware_concurrency () / 2) };
	uint32_t bootstrap_frontier_request_count{ 1024 * 1024 };
	nano::websocket::config websocket_config;
	nano::diagnostics_config diagnostics_config;
	std::size_t confirmation_history_size;
	std::string callback_address;
	uint16_t callback_port;
	std::string callback_target;
	bool allow_local_peers;
	nano::stat_config stat_config;
	nano::ipc::ipc_config ipc_config;
	std::string external_address;
	uint16_t external_port;
	std::chrono::milliseconds block_processor_batch_max_time{ network_params.network.is_dev_network () ? std::chrono::milliseconds (500) : std::chrono::milliseconds (5000) };
	std::chrono::seconds unchecked_cutoff_time;
	/** Timeout for initiated async operations */
	std::chrono::seconds tcp_io_timeout;
	std::chrono::nanoseconds pow_sleep_interval;
	std::size_t active_elections_size;
	std::size_t active_elections_hinted_limit_percentage; // Limit of hinted elections as percentage of active_elections_size
	/** Default maximum incoming TCP connections, including realtime network & bootstrap */
	unsigned tcp_incoming_connections_max;
	bool use_memory_pools;
	static std::chrono::minutes constexpr wallet_backup_interval = std::chrono::minutes (5);
	/** Default outbound traffic shaping is 10MB/s */
	std::size_t bandwidth_limit;
	/** By default, allow bursts of 15MB/s (not sustainable) */
	double bandwidth_limit_burst_ratio{ 3. };
	std::size_t bootstrap_bandwidth_limit;
	double bootstrap_bandwidth_burst_ratio;
	std::chrono::milliseconds conf_height_processor_batch_min_time;
	bool backup_before_upgrade{ false };
	double max_work_generate_multiplier;
	uint32_t max_queued_requests;
	std::chrono::seconds max_pruning_age;
	uint64_t max_pruning_depth;
	nano::lmdb_config lmdb_config;
	nano::frontiers_confirmation_mode frontiers_confirmation{ nano::frontiers_confirmation_mode::automatic };
	/** Number of accounts per second to process when doing backlog population scan */
	unsigned backlog_scan_batch_size;
	/** Number of times per second to run backlog population batches. Number of accounts per single batch is `backlog_scan_batch_size / backlog_scan_frequency` */
	unsigned backlog_scan_frequency;
	nano::frontiers_confirmation_mode deserialize_frontiers_confirmation (std::string const &);
	/** Entry is ignored if it cannot be parsed as a valid address:port */
	void deserialize_address (std::string const &, std::vector<std::pair<std::string, uint16_t>> &) const;
};

class node_flags final
{
public:
	node_flags ();
	node_flags (node_flags const & other_a);
	node_flags (node_flags && other_a);
	~node_flags ();
	node_flags & operator= (node_flags const & other_a);
	node_flags & operator= (node_flags && other_a);
	std::vector<std::string> config_overrides () const;
	void set_config_overrides (const std::vector<std::string> & overrides);
	std::vector<std::string> rpc_config_overrides () const;
	void set_rpc_overrides (const std::vector<std::string> & overrides);
	bool disable_add_initial_peers () const;
	void set_disable_add_initial_peers (bool value);
	bool disable_backup () const;
	void set_disable_backup (bool value);
	bool disable_lazy_bootstrap () const;
	void set_disable_lazy_bootstrap (bool value);
	bool disable_legacy_bootstrap () const;
	void set_disable_legacy_bootstrap (bool value);
	bool disable_wallet_bootstrap () const;
	void set_disable_wallet_bootstrap (bool value);
	bool disable_bootstrap_listener () const;
	void set_disable_bootstrap_listener (bool value);
	bool disable_bootstrap_bulk_pull_server () const;
	void set_disable_bootstrap_bulk_pull_server (bool value);
	bool disable_bootstrap_bulk_push_client () const;
	void set_disable_bootstrap_bulk_push_client (bool value);
	bool disable_ongoing_bootstrap () const; // For testing only
	void set_disable_ongoing_bootstrap (bool value);
	bool disable_rep_crawler () const;
	void set_disable_rep_crawler (bool value);
	bool disable_request_loop () const; // For testing only
	void set_disable_request_loop (bool value);
	bool disable_tcp_realtime () const;
	void set_disable_tcp_realtime (bool value);
	bool disable_udp () const;
	void set_disable_udp (bool value);
	bool disable_unchecked_cleanup () const;
	void set_disable_unchecked_cleanup (bool value);
	bool disable_unchecked_drop () const;
	void set_disable_unchecked_drop (bool value);
	bool disable_providing_telemetry_metrics () const;
	void set_disable_providing_telemetry_metrics (bool value);
	bool disable_ongoing_telemetry_requests () const;
	void set_disable_ongoing_telemetry_requests (bool value);
	bool disable_initial_telemetry_requests () const;
	void set_disable_initial_telemetry_requests (bool value);
	bool disable_block_processor_unchecked_deletion () const;
	void set_disable_block_processor_unchecked_deletion (bool value);
	bool disable_block_processor_republishing () const;
	void set_disable_block_processor_republishing (bool value);
	bool allow_bootstrap_peers_duplicates () const;
	void set_allow_bootstrap_peers_duplicates (bool value);
	bool disable_max_peers_per_ip () const; // For testing only
	void set_disable_max_peers_per_ip (bool value);
	bool disable_max_peers_per_subnetwork () const; // For testing only
	void set_disable_max_peers_per_subnetwork (bool value);
	bool force_use_write_database_queue () const; // For testing only. RocksDB does not use the database queue, but some tests rely on it being used.
	void set_force_use_write_database_queue (bool value);
	bool disable_search_pending () const; // For testing only
	void set_disable_search_pending (bool value);
	bool enable_pruning () const;
	void set_enable_pruning (bool value);
	bool fast_bootstrap () const;
	void set_fast_bootstrap (bool value);
	bool read_only () const;
	void set_read_only (bool value);
	bool disable_connection_cleanup () const;
	void set_disable_connection_cleanup (bool value);
	nano::confirmation_height_mode confirmation_height_processor_mode () const;
	void set_confirmation_height_processor_mode (nano::confirmation_height_mode mode);
	nano::generate_cache generate_cache () const;
	void set_generate_cache (nano::generate_cache const & cache);
	bool inactive_node () const;
	void set_inactive_node (bool value);
	std::size_t block_processor_batch_size () const;
	void set_block_processor_batch_size (std::size_t size);
	std::size_t block_processor_full_size () const;
	void set_block_processor_full_size (std::size_t size);
	std::size_t block_processor_verification_size () const;
	void set_block_processor_verification_size (std::size_t size);
	std::size_t inactive_votes_cache_size () const;
	void set_inactive_votes_cache_size (std::size_t size);
	std::size_t vote_processor_capacity () const;
	void set_vote_processor_capacity (std::size_t size);
	std::size_t bootstrap_interval () const; // For testing only
	void set_bootstrap_interval (std::size_t size);
	rsnano::NodeFlagsHandle * handle;

private:
	rsnano::NodeFlagsDto flags_dto () const;
	void set_flag (std::function<void (rsnano::NodeFlagsDto &)> const & callback);
};
}
