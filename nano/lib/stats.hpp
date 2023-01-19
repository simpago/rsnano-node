#pragma once

#include <nano/lib/errors.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/utility.hpp>

#include <boost/circular_buffer.hpp>

#include <chrono>
#include <initializer_list>
#include <map>
#include <memory>
#include <mutex>
#include <string>

namespace nano
{
class node;
class tomlconfig;
class jsonconfig;
/**
 * Serialize and deserialize the 'statistics' node from config.json
 * All configuration values have defaults. In particular, file logging of statistics
 * is disabled by default.
 */
class stat_config final
{
public:
	void load_dto (rsnano::StatConfigDto & dto);
	[[nodiscard]] rsnano::StatConfigDto to_dto () const;
	/** Reads the JSON statistics node */
	nano::error deserialize_toml (nano::tomlconfig & toml);

	/** If true, sampling of counters is enabled */
	bool sampling_enabled{ false };

	/** How many sample intervals to keep in the ring buffer */
	size_t capacity{ 0 };

	/** Sample interval in milliseconds */
	size_t interval{ 0 };

	/** How often to log sample array, in milliseconds. Default is 0 (no logging) */
	size_t log_interval_samples{ 0 };

	/** How often to log counters, in milliseconds. Default is 0 (no logging) */
	size_t log_interval_counters{ 0 };

	/** Maximum number of log outputs before rotating the file */
	size_t log_rotation_count{ 100 };

	/** If true, write headers on each counter or samples writeout. The header contains log type and the current wall time. */
	bool log_headers{ true };

	/** Filename for the counter log  */
	std::string log_counters_filename{ "counters.stat" };

	/** Filename for the sampling log */
	std::string log_samples_filename{ "samples.stat" };
};

/** Log sink interface */
class stat_log_sink
{
public:
	explicit stat_log_sink (rsnano::StatLogSinkHandle * handle_a);
	virtual ~stat_log_sink ();

public:
	/**
	 * Returns the object representation of the log result. The type depends on the sink used.
	 * @returns Object, or nullptr if no object result is available.
	 */
	void * to_object ();

	rsnano::StatLogSinkHandle * handle;
};

/**
 * Collects counts and samples for inbound and outbound traffic, blocks, errors, and so on.
 * Stats can be queried and observed on a type level (such as message and ledger) as well as a more
 * specific detail level (such as send blocks)
 */
class stat final
{
public:
	/** Primary statistics type */
	enum class
	type : uint8_t
	{
		traffic_udp,
		traffic_tcp,
		error,
		message,
		block,
		ledger,
		rollback,
		bootstrap,
		tcp_server,
		vote,
		election,
		http_callback,
		peering,
		ipc,
		tcp,
		udp,
		confirmation_height,
		confirmation_observer,
		drop,
		aggregator,
		requests,
		filter,
		telemetry,
		vote_generator,
		vote_cache,
		hinting,
		blockprocessor,
		bootstrap_server,
		active,
		backlog,
	};

	/** Optional detail type */
	enum class detail : uint8_t
	{
		all = 0,

		// common
		loop,
		total,

		// processing queue
		queue,
		overfill,
		batch,

		// error specific
		bad_sender,
		insufficient_work,
		http_callback,
		unreachable_host,
		invalid_network,

		// confirmation_observer specific
		active_quorum,
		active_conf_height,
		inactive_conf_height,

		// ledger, block, bootstrap
		send,
		receive,
		open,
		change,
		state_block,
		epoch_block,
		fork,
		old,
		gap_previous,
		gap_source,
		rollback_failed,
		progress,
		bad_signature,
		negative_spend,
		unreceivable,
		gap_epoch_open_pending,
		opened_burn_account,
		balance_mismatch,
		representative_mismatch,
		block_position,

		// message specific
		not_a_type,
		invalid,
		keepalive,
		publish,
		republish_vote,
		confirm_req,
		confirm_ack,
		node_id_handshake,
		telemetry_req,
		telemetry_ack,
		asc_pull_req,
		asc_pull_ack,

		// bootstrap, callback
		initiate,
		initiate_legacy_age,
		initiate_lazy,
		initiate_wallet_lazy,

		// bootstrap specific
		bulk_pull,
		bulk_pull_account,
		bulk_pull_deserialize_receive_block,
		bulk_pull_error_starting_request,
		bulk_pull_failed_account,
		bulk_pull_receive_block_failure,
		bulk_pull_request_failure,
		bulk_push,
		frontier_req,
		frontier_confirmation_failed,
		frontier_confirmation_successful,
		error_socket_close,
		request_underflow,

		// vote specific
		vote_valid,
		vote_replay,
		vote_indeterminate,
		vote_invalid,
		vote_overflow,

		// election specific
		vote_new,
		vote_processed,
		vote_cached,
		late_block,
		late_block_seconds,
		election_start,
		election_confirmed_all,
		election_block_conflict,
		election_difficulty_update,
		election_drop_expired,
		election_drop_overflow,
		election_drop_all,
		election_restart,
		election_confirmed,
		election_not_confirmed,
		election_hinted_overflow,
		election_hinted_started,
		election_hinted_confirmed,
		election_hinted_drop,
		generate_vote,
		generate_vote_normal,
		generate_vote_final,

		// udp
		blocking,
		overflow,
		invalid_header,
		invalid_message_type,
		invalid_keepalive_message,
		invalid_publish_message,
		invalid_confirm_req_message,
		invalid_confirm_ack_message,
		invalid_node_id_handshake_message,
		invalid_telemetry_req_message,
		invalid_telemetry_ack_message,
		invalid_bulk_pull_message,
		invalid_bulk_pull_account_message,
		invalid_frontier_req_message,
		invalid_asc_pull_req_message,
		invalid_asc_pull_ack_message,
		message_too_big,
		outdated_version,
		udp_max_per_ip,
		udp_max_per_subnetwork,

		// tcp
		tcp_accept_success,
		tcp_accept_failure,
		tcp_write_drop,
		tcp_write_no_socket_drop,
		tcp_excluded,
		tcp_max_per_ip,
		tcp_max_per_subnetwork,
		tcp_silent_connection_drop,
		tcp_io_timeout_drop,
		tcp_connect_error,
		tcp_read_error,
		tcp_write_error,

		// ipc
		invocations,

		// peering
		handshake,

		// confirmation height
		blocks_confirmed,
		blocks_confirmed_unbounded,
		blocks_confirmed_bounded,

		// [request] aggregator
		aggregator_accepted,
		aggregator_dropped,

		// requests
		requests_cached_hashes,
		requests_generated_hashes,
		requests_cached_votes,
		requests_generated_votes,
		requests_cached_late_hashes,
		requests_cached_late_votes,
		requests_cannot_vote,
		requests_unknown,

		// duplicate
		duplicate_publish,

		// telemetry
		invalid_signature,
		different_genesis_hash,
		node_id_mismatch,
		request_within_protection_cache_zone,
		no_response_received,
		unsolicited_telemetry_ack,
		failed_send_telemetry_req,

		// vote generator
		generator_broadcasts,
		generator_replies,
		generator_replies_discarded,
		generator_spacing,

		// hinting
		hinted,
		insert_failed,
		missing_block,

		// bootstrap server
		response,
		write_drop,
		write_error,
		blocks,
		drop,
		bad_count,
		response_blocks,
		response_account_info,
		channel_full,

		// backlog
		activated,
	};

	/** Direction of the stat. If the direction is irrelevant, use in */
	enum class dir : uint8_t
	{
		in,
		out
	};

	/** Constructor using the default config values */
	stat ();
	explicit stat (rsnano::StatHandle * handle_a);
	~stat ();

	/**
	 * Initialize stats with a config.
	 * @param config Configuration object; deserialized from config.json
	 */
	explicit stat (nano::stat_config config);
	stat (nano::stat const &) = delete;
	stat (nano::stat &&) = delete;

	/**
	 * Call this to override the default sample interval and capacity, for a specific stat entry.
	 * This must be called before any stat entries are added, as part of the node initialiation.
	 */
	void configure (stat::type type, stat::detail detail, stat::dir dir, size_t interval, size_t capacity);

	/** Increments the given counter */
	void inc (stat::type type, stat::dir dir = stat::dir::in);

	/** Increments the counter for \detail, but doesn't update at the type level */
	void inc_detail_only (stat::type type, stat::detail detail, stat::dir dir = stat::dir::in);

	/** Increments the given counter */
	void inc (stat::type type, stat::detail detail, stat::dir dir = stat::dir::in);

	/** Adds \p value to the given counter */
	void add (stat::type type, stat::dir dir, uint64_t value);

	/**
	 * Add \p value to stat. If sampling is configured, this will update the current sample and
	 * call any sample observers if the interval is over.
	 *
	 * @param type Main statistics type
	 * @param detail Detail type, or detail::none to register on type-level only
	 * @param dir Direction
	 * @param value The amount to add
	 * @param detail_only If true, only update the detail-level counter
	 */
	void add (stat::type type, stat::detail detail, stat::dir dir, uint64_t value, bool detail_only = false);

	/** Returns current value for the given counter at the type level */
	uint64_t count (stat::type type, stat::dir dir = stat::dir::in);

	/** Returns current value for the given counter at the detail level */
	uint64_t count (stat::type type, stat::detail detail, stat::dir dir = stat::dir::in);

	/** Returns the number of seconds since clear() was last called, or node startup if it's never called. */
	std::chrono::seconds last_reset ();

	/** Clear all stats */
	void clear ();

	/** Log counters to the given log link */
	void log_counters (stat_log_sink & sink);

	/** Log samples to the given log sink */
	void log_samples (stat_log_sink & sink);

	/** Returns a new JSON log sink */
	[[nodiscard]] std::unique_ptr<stat_log_sink> log_sink_json () const;

	/** Returns string representation of type */
	static std::string type_to_string (stat::type type);

	/** Returns string representation of detail */
	static std::string detail_to_string (stat::detail detail);

	/** Returns string representation of dir */
	static std::string dir_to_string (stat::dir detail);

	/** Stop stats being output */
	void stop ();

	rsnano::StatHandle * handle;
};
}
