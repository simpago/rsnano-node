#pragma once

#include "nano/lib/rsnano.hpp"

#include <nano/node/bootstrap/bootstrap_connections.hpp>
#include <nano/node/common.hpp>

#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/member.hpp>
#include <boost/multi_index/ordered_index.hpp>
#include <boost/multi_index_container.hpp>
#include <boost/thread/thread.hpp>

namespace nano::store
{
class transaction;
}

namespace nano
{
class node;

class bootstrap_connections;
enum class bootstrap_mode
{
	legacy,
	lazy,
	wallet_lazy,
	ascending
};
enum class sync_result
{
	success,
	error,
	fork
};
class cached_pulls final
{
public:
	std::chrono::steady_clock::time_point time;
	nano::uint512_union account_head;
	nano::block_hash new_head;
};
class pulls_cache final
{
public:
	pulls_cache ();
	pulls_cache (rsnano::PullsCacheHandle * handle);
	pulls_cache (pulls_cache const &) = delete;
	pulls_cache (pulls_cache &&) = delete;
	~pulls_cache ();
	void add (nano::pull_info const &);
	void update_pull (nano::pull_info &);
	void remove (nano::pull_info const &);
	size_t size ();
	static size_t element_size ();
	rsnano::PullsCacheHandle * handle;
};

/**
 * Container for bootstrap sessions that are active. Owned by bootstrap_initiator.
 */
class bootstrap_attempts final
{
public:
	bootstrap_attempts ();
	explicit bootstrap_attempts (rsnano::BootstrapAttemptsHandle * handle);
	bootstrap_attempts (bootstrap_attempts const &) = delete;
	bootstrap_attempts (bootstrap_attempts &&) = delete;
	~bootstrap_attempts () noexcept;
	std::size_t size ();
	uint64_t total_attempts () const;
	boost::property_tree::ptree attempts_information ();
	rsnano::BootstrapAttemptsHandle * handle;
};

class bootstrap_attempt_lazy;
class bootstrap_attempt_wallet;
/**
 * Client side portion to initiate bootstrap sessions. Prevents multiple legacy-type bootstrap sessions from being started at the same time. Does permit
 * lazy/wallet bootstrap sessions to overlap with legacy sessions.
 */
class bootstrap_initiator final
{
public:
	rsnano::BootstrapInitiatorHandle * handle;

	explicit bootstrap_initiator (rsnano::BootstrapInitiatorHandle * handle);
	bootstrap_initiator (nano::bootstrap_initiator const &) = delete;
	~bootstrap_initiator ();
	void bootstrap (nano::endpoint const &, std::string id_a = "");
	void bootstrap (bool force = false, std::string id_a = "", uint32_t const frontiers_age_a = std::numeric_limits<uint32_t>::max (), nano::account const & start_account_a = nano::account{});
	bool bootstrap_lazy (nano::hash_or_account const &, bool force = false, std::string id_a = "");
	bool in_progress ();
	nano::bootstrap_attempts attempts;
	std::shared_ptr<nano::bootstrap_connections> connections;
	std::shared_ptr<nano::bootstrap_attempt> current_attempt ();
	std::shared_ptr<nano::bootstrap_attempt_lazy> current_lazy_attempt ();
	std::shared_ptr<nano::bootstrap_attempt_wallet> current_wallet_attempt ();
	rsnano::BootstrapInitiatorHandle * get_handle () const;
	nano::pulls_cache cache;
	void stop ();
};

/**
 * Defines the numeric values for the bootstrap feature.
 */
class bootstrap_limits final
{
public:
	static constexpr double bootstrap_connection_scale_target_blocks = 10000.0;
	static constexpr double bootstrap_connection_warmup_time_sec = 5.0;
	static constexpr double bootstrap_minimum_blocks_per_sec = 10.0;
	static constexpr double bootstrap_minimum_elapsed_seconds_blockrate = 0.02;
	static constexpr double bootstrap_minimum_frontier_blocks_per_sec = 1000.0;
	static constexpr double bootstrap_minimum_termination_time_sec = 30.0;
	static constexpr unsigned bootstrap_max_new_connections = 32;
	static constexpr unsigned requeued_pulls_limit = 256;
	static constexpr unsigned requeued_pulls_limit_dev = 1;
	static constexpr unsigned requeued_pulls_processed_blocks_factor = 4096;
	static constexpr unsigned bulk_push_cost_limit = 200;
	static constexpr std::chrono::seconds lazy_flush_delay_sec = std::chrono::seconds (5);
	static constexpr uint64_t lazy_batch_pull_count_resize_blocks_limit = 4 * 1024 * 1024;
	static constexpr double lazy_batch_pull_count_resize_ratio = 2.0;
	static constexpr std::size_t lazy_blocks_restart_limit = 1024 * 1024;
};
}
