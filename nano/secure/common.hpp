#pragma once

#include <nano/lib/blockbuilders.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/lib/config.hpp>
#include <nano/lib/epoch.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/rep_weights.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/stats.hpp>
#include <nano/lib/utility.hpp>

#include <boost/iterator/transform_iterator.hpp>
#include <boost/optional/optional.hpp>
#include <boost/property_tree/ptree_fwd.hpp>
#include <boost/variant/variant.hpp>

#include <array>
#include <unordered_map>

namespace rsnano
{
class VoteHandle;
class VoteUniquerHandle;
}

namespace boost
{
template <>
struct hash<::nano::uint256_union>
{
	size_t operator() (::nano::uint256_union const & value_a) const
	{
		return std::hash<::nano::uint256_union> () (value_a);
	}
};

template <>
struct hash<::nano::block_hash>
{
	size_t operator() (::nano::block_hash const & value_a) const
	{
		return std::hash<::nano::block_hash> () (value_a);
	}
};

template <>
struct hash<::nano::hash_or_account>
{
	size_t operator() (::nano::hash_or_account const & data_a) const
	{
		return std::hash<::nano::hash_or_account> () (data_a);
	}
};

template <>
struct hash<::nano::public_key>
{
	size_t operator() (::nano::public_key const & value_a) const
	{
		return std::hash<::nano::public_key> () (value_a);
	}
};
template <>
struct hash<::nano::uint512_union>
{
	size_t operator() (::nano::uint512_union const & value_a) const
	{
		return std::hash<::nano::uint512_union> () (value_a);
	}
};
template <>
struct hash<::nano::qualified_root>
{
	size_t operator() (::nano::qualified_root const & value_a) const
	{
		return std::hash<::nano::qualified_root> () (value_a);
	}
};
template <>
struct hash<::nano::root>
{
	size_t operator() (::nano::root const & value_a) const
	{
		return std::hash<::nano::root> () (value_a);
	}
};
}
namespace nano
{
/**
 * A key pair. The private key is generated from the random pool, or passed in
 * as a hex string. The public key is derived using ed25519.
 */
class keypair
{
public:
	keypair ();
	explicit keypair (std::string const &);
	explicit keypair (nano::raw_key &&);
	keypair (nano::keypair const &);
	keypair (nano::raw_key const & priv_key_a, nano::public_key const & pub_key_a);
	nano::public_key pub;
	nano::raw_key prv;
};

/**
 * Latest information about an account
 */
class account_info final
{
public:
	account_info ();
	account_info (nano::block_hash const &, nano::account const &, nano::block_hash const &, nano::amount const &, uint64_t, uint64_t, epoch);
	account_info (account_info const &);
	account_info (account_info &&);
	~account_info ();
	account_info & operator= (account_info const &);
	bool serialize (nano::stream &) const;
	bool deserialize (nano::stream &);
	bool operator== (nano::account_info const &) const;
	bool operator!= (nano::account_info const &) const;
	size_t db_size () const;
	nano::epoch epoch () const;
	nano::block_hash head () const;
	nano::account representative () const;
	nano::block_hash open_block () const;
	nano::amount balance () const;
	uint64_t modified () const;
	uint64_t block_count () const;
	rsnano::AccountInfoHandle * handle;
};

/**
 * Information on an uncollected send
 */
class pending_info final
{
public:
	pending_info () = default;
	pending_info (nano::account const &, nano::amount const &, nano::epoch);
	size_t db_size () const;
	bool deserialize (nano::stream &);
	bool operator== (nano::pending_info const &) const;
	nano::account source{};
	nano::amount amount{ 0 };
	nano::epoch epoch{ nano::epoch::epoch_0 };
};
class pending_key final
{
public:
	pending_key () = default;
	pending_key (nano::account const &, nano::block_hash const &);
	bool deserialize (nano::stream &);
	bool operator== (nano::pending_key const &) const;
	nano::account const & key () const;
	nano::account account{};
	nano::block_hash hash{ 0 };
};

class endpoint_key final
{
public:
	endpoint_key () = default;

	/*
	 * @param address_a This should be in network byte order
	 * @param port_a This should be in host byte order
	 */
	endpoint_key (std::array<uint8_t, 16> const & address_a, uint16_t port_a);

	/*
	 * @return The ipv6 address in network byte order
	 */
	std::array<uint8_t, 16> const & address_bytes () const;

	/*
	 * @return The port in host byte order
	 */
	uint16_t port () const;

private:
	// Both stored internally in network byte order
	std::array<uint8_t, 16> address;
	uint16_t network_port{ 0 };
};

enum class no_value
{
	dummy
};

class unchecked_key final
{
public:
	unchecked_key () = default;
	explicit unchecked_key (nano::hash_or_account const & dependency);
	unchecked_key (nano::hash_or_account const &, nano::block_hash const &);
	unchecked_key (nano::uint512_union const &);
	rsnano::UncheckedKeyDto to_dto () const;
	bool deserialize (nano::stream &);
	bool operator== (nano::unchecked_key const &) const;
	bool operator< (nano::unchecked_key const &) const;
	nano::block_hash const & key () const;
	nano::block_hash previous{ 0 };
	nano::block_hash hash{ 0 };
};

/**
 * Information on an unchecked block
 */
class unchecked_info final
{
public:
	unchecked_info ();
	unchecked_info (std::shared_ptr<nano::block> const &);
	unchecked_info (nano::unchecked_info const &);
	unchecked_info (nano::unchecked_info &&);
	unchecked_info (rsnano::UncheckedInfoHandle * handle_a);
	~unchecked_info ();
	nano::unchecked_info & operator= (const nano::unchecked_info &);
	void serialize (nano::stream &) const;
	bool deserialize (nano::stream &);
	uint64_t modified () const;
	std::shared_ptr<nano::block> get_block () const;
	rsnano::UncheckedInfoHandle * handle;
};

class block_info final
{
public:
	block_info () = default;
	block_info (nano::account const &, nano::amount const &);
	nano::account account{};
	nano::amount balance{ 0 };
};

class confirmation_height_info final
{
public:
	confirmation_height_info ();
	confirmation_height_info (uint64_t, nano::block_hash const &);

	void serialize (nano::stream &) const;
	bool deserialize (nano::stream &);

	/** height of the cemented frontier */
	uint64_t height () const;

	/** hash of the highest cemented block, the cemented/confirmed frontier */
	nano::block_hash frontier () const;
	rsnano::ConfirmationHeightInfoDto dto;
};

namespace confirmation_height
{
	/** When the uncemented count (block count - cemented count) is less than this use the unbounded processor */
	uint64_t const unbounded_cutoff{ 16384 };
}

using vote_blocks_vec_iter = std::vector<nano::block_hash>::const_iterator;
class iterate_vote_blocks_as_hash final
{
public:
	iterate_vote_blocks_as_hash () = default;
	nano::block_hash operator() (nano::block_hash const & item) const;
};

class vote final
{
public:
	vote ();
	vote (nano::account const &);
	vote (rsnano::VoteHandle * handle_a);
	vote (nano::vote const &);
	vote (nano::vote &&);
	vote (bool &, nano::stream &);
	vote (nano::account const &, nano::raw_key const &, uint64_t timestamp, uint8_t duration, std::vector<nano::block_hash> const &);
	~vote ();
	std::string hashes_string () const;
	nano::block_hash hash () const;
	nano::block_hash full_hash () const;
	bool operator== (nano::vote const &) const;
	bool operator!= (nano::vote const &) const;
	void serialize (nano::stream &) const;
	void serialize_json (boost::property_tree::ptree & tree) const;
	/**
	 * Deserializes a vote from the bytes in `stream'
	 * Returns true if there was an error
	 */
	bool deserialize (nano::stream &);
	bool validate () const;
	uint64_t timestamp () const;
	uint8_t duration_bits () const;
	nano::account account () const;
	nano::signature signature () const;
	std::chrono::milliseconds duration () const;
	std::vector<nano::block_hash> hashes () const;
	void flip_signature_bit_0 ();
	rsnano::VoteHandle * get_handle () const;
	// gets the pointer to the block data within Rust;
	const void * get_rust_data_pointer () const;
	static uint64_t constexpr timestamp_max = { 0xffff'ffff'ffff'fff0ULL };
	static uint64_t constexpr timestamp_min = { 0x0000'0000'0000'0010ULL };
	static uint8_t constexpr duration_max = { 0x0fu };

private:
	rsnano::VoteHandle * handle{ nullptr };

public:
	static std::string const hash_prefix;
};
/**
 * This class serves to find and return unique variants of a vote in order to minimize memory usage
 */
class vote_uniquer final
{
public:
	using value_type = std::pair<nano::block_hash const, std::weak_ptr<nano::vote>>;

	vote_uniquer (nano::block_uniquer &);
	vote_uniquer (nano::vote_uniquer &&) = delete;
	vote_uniquer (const nano::vote_uniquer &) = delete;
	~vote_uniquer ();
	std::shared_ptr<nano::vote> unique (std::shared_ptr<nano::vote> const &);
	size_t size ();
	vote_uniquer & operator= (vote_uniquer const &) = delete;
	rsnano::VoteUniquerHandle * handle;
};

std::unique_ptr<container_info_component> collect_container_info (vote_uniquer & vote_uniquer, std::string const & name);

enum class vote_code
{
	invalid, // Vote is not signed correctly
	replay, // Vote does not have the highest timestamp, it's a replay
	vote, // Vote has the highest timestamp
	indeterminate // Unknown if replay or vote
};

enum class process_result
{
	progress, // Hasn't been seen before, signed correctly
	bad_signature, // Signature was bad, forged or transmission error
	old, // Already seen and was valid
	negative_spend, // Malicious attempt to spend a negative amount
	fork, // Malicious fork based on previous
	unreceivable, // Source block doesn't exist, has already been received, or requires an account upgrade (epoch blocks)
	gap_previous, // Block marked as previous is unknown
	gap_source, // Block marked as source is unknown
	gap_epoch_open_pending, // Block marked as pending blocks required for epoch open block are unknown
	opened_burn_account, // Block attempts to open the burn account
	balance_mismatch, // Balance and amount delta don't match
	representative_mismatch, // Representative is changed when it is not allowed
	block_position, // This block cannot follow the previous block
	insufficient_work // Insufficient work for this block, even though it passed the minimal validation
};
class process_return final
{
public:
	nano::process_result code;
};
enum class tally_result
{
	vote,
	changed,
	confirm
};

nano::stat::detail to_stat_detail (process_result);

class network_params;

class NetworkParamsDtoWrapper
{
public:
	NetworkParamsDtoWrapper (rsnano::NetworkParamsDto dto_a) :
		dto{ dto_a }
	{
	}
	NetworkParamsDtoWrapper (NetworkParamsDtoWrapper const &) = delete;
	NetworkParamsDtoWrapper (NetworkParamsDtoWrapper && other_a)
	{
		dto = other_a.dto;
		other_a.moved = true;
	}
	~NetworkParamsDtoWrapper ()
	{
		if (!moved)
		{
			rsnano::rsn_block_destroy (dto.ledger.genesis);
			rsnano::rsn_block_destroy (dto.ledger.nano_beta_genesis);
			rsnano::rsn_block_destroy (dto.ledger.nano_dev_genesis);
			rsnano::rsn_block_destroy (dto.ledger.nano_live_genesis);
			rsnano::rsn_block_destroy (dto.ledger.nano_test_genesis);
		}
	}
	rsnano::NetworkParamsDto dto;
	bool moved{ false };
};

/** Genesis keys and ledger constants for network variants */
class ledger_constants
{
public:
	ledger_constants () = delete;
	ledger_constants (nano::work_thresholds work, nano::networks network_a);
	ledger_constants (rsnano::LedgerConstantsDto const & dto);
	void read_dto (rsnano::LedgerConstantsDto const & dto);
	nano::work_thresholds work;
	nano::keypair zero_key;
	nano::account nano_beta_account;
	nano::account nano_live_account;
	nano::account nano_test_account;
	std::shared_ptr<nano::block> nano_dev_genesis;
	std::shared_ptr<nano::block> nano_beta_genesis;
	std::shared_ptr<nano::block> nano_live_genesis;
	std::shared_ptr<nano::block> nano_test_genesis;
	std::shared_ptr<nano::block> genesis;
	nano::uint128_t genesis_amount;
	nano::account burn_account;
	nano::account nano_dev_final_votes_canary_account;
	nano::account nano_beta_final_votes_canary_account;
	nano::account nano_live_final_votes_canary_account;
	nano::account nano_test_final_votes_canary_account;
	nano::account final_votes_canary_account;
	uint64_t nano_dev_final_votes_canary_height;
	uint64_t nano_beta_final_votes_canary_height;
	uint64_t nano_live_final_votes_canary_height;
	uint64_t nano_test_final_votes_canary_height;
	uint64_t final_votes_canary_height;
	nano::epochs epochs;
	rsnano::LedgerConstantsDto to_dto () const;
};

namespace dev
{
	extern nano::keypair genesis_key;
	extern nano::network_params network_params;
	extern nano::ledger_constants & constants;
	extern std::shared_ptr<nano::block> & genesis;
}

/** Constants which depend on random values (always used as singleton) */
class hardened_constants
{
public:
	static hardened_constants & get ();

	nano::account not_an_account;
	nano::uint128_union random_128;

private:
	hardened_constants ();
};

/** Node related constants whose value depends on the active network */
class node_constants
{
public:
	node_constants () = default;
	node_constants (rsnano::NodeConstantsDto const &);
	void read_dto (rsnano::NodeConstantsDto const &);
	rsnano::NodeConstantsDto to_dto () const;
	std::chrono::minutes backup_interval;
	std::chrono::seconds search_pending_interval;
	std::chrono::minutes unchecked_cleaning_interval;
	std::chrono::milliseconds process_confirmed_interval;

	/** The maximum amount of samples for a 2 week period on live or 1 day on beta */
	uint64_t max_weight_samples;
	uint64_t weight_period;
};

/** Voting related constants whose value depends on the active network */
class voting_constants
{
public:
	voting_constants () = default;
	voting_constants (rsnano::VotingConstantsDto const & dto);
	size_t max_cache;
	std::chrono::seconds delay;
	rsnano::VotingConstantsDto to_dto () const;
};

/** Port-mapping related constants whose value depends on the active network */
class portmapping_constants
{
public:
	portmapping_constants () = default;
	portmapping_constants (nano::network_constants & network_constants);
	portmapping_constants (rsnano::PortmappingConstantsDto const & dto);
	// Timeouts are primes so they infrequently happen at the same time
	std::chrono::seconds lease_duration;
	std::chrono::seconds health_check_period;
	rsnano::PortmappingConstantsDto to_dto () const;
};

/** Bootstrap related constants whose value depends on the active network */
class bootstrap_constants
{
public:
	bootstrap_constants () = default;
	bootstrap_constants (rsnano::BootstrapConstantsDto const & dto);
	void read_dto (rsnano::BootstrapConstantsDto const & dto);
	uint32_t lazy_max_pull_blocks;
	uint32_t lazy_min_pull_blocks;
	unsigned frontier_retry_limit;
	unsigned lazy_retry_limit;
	unsigned lazy_destinations_retry_limit;
	std::chrono::milliseconds gap_cache_bootstrap_start_interval;
	uint32_t default_frontiers_age_seconds;
	rsnano::BootstrapConstantsDto to_dto () const;
};

/** Constants whose value depends on the active network */
class network_params
{
public:
	network_params () = delete;
	/** Populate values based on \p network_a */
	network_params (nano::networks network_a);
	network_params (rsnano::NetworkParamsDto const & dto);

	nano::NetworkParamsDtoWrapper to_dto () const;

	unsigned kdf_work;
	nano::work_thresholds work;
	nano::network_constants network;
	nano::ledger_constants ledger;
	nano::voting_constants voting;
	nano::node_constants node;
	nano::portmapping_constants portmapping;
	nano::bootstrap_constants bootstrap;
};

enum class confirmation_height_mode
{
	automatic,
	unbounded,
	bounded
};

/* Holds flags for various cacheable data. For most CLI operations caching is unnecessary
 * (e.g getting the cemented block count) so it can be disabled for performance reasons. */
class generate_cache
{
public:
	generate_cache ();
	generate_cache (rsnano::GenerateCacheHandle * handle_a);
	generate_cache (generate_cache const &);
	generate_cache (generate_cache && other_a) noexcept;
	~generate_cache ();
	generate_cache & operator= (generate_cache const & other_a);
	generate_cache & operator= (generate_cache && other_a);
	bool reps () const;
	void enable_reps (bool enable);
	bool cemented_count () const;
	void enable_cemented_count (bool enable);
	void enable_unchecked_count (bool enable);
	bool account_count () const;
	void enable_account_count (bool enable);
	bool block_count () const;
	void enable_block_count (bool enable);
	void enable_all ();
	rsnano::GenerateCacheHandle * handle;
};

/* Holds an in-memory cache of various counts */
class ledger_cache
{
public:
	ledger_cache ();
	ledger_cache (rsnano::LedgerCacheHandle * handle_a);
	ledger_cache (ledger_cache &&);
	~ledger_cache ();
	ledger_cache (ledger_cache const &) = delete;
	ledger_cache & operator= (ledger_cache && other_a);
	nano::rep_weights & rep_weights ();
	uint64_t cemented_count () const;
	void add_cemented (uint64_t count);
	uint64_t block_count () const;
	void add_blocks (uint64_t count);
	void remove_blocks (uint64_t count);
	uint64_t pruned_count () const;
	void add_pruned (uint64_t count);
	uint64_t account_count () const;
	void add_accounts (uint64_t count);
	void remove_accounts (uint64_t count);
	bool final_votes_confirmation_canary () const;
	void set_final_votes_confirmation_canary (bool canary);
	rsnano::LedgerCacheHandle * handle;

private:
	nano::rep_weights rep_weights_m;
};

/* Defines the possible states for an election to stop in */
enum class election_status_type : uint8_t
{
	ongoing = 0,
	active_confirmed_quorum = 1,
	active_confirmation_height = 2,
	inactive_confirmation_height = 3,
	stopped = 5
};

/* Holds a summary of an election */
class election_status final
{
public:
	election_status ();
	election_status (rsnano::ElectionStatusHandle * handle);
	election_status (std::shared_ptr<nano::block> const & winner_a);
	election_status (election_status &&) = delete;
	election_status (election_status const &);
	~election_status ();
	nano::election_status & operator= (const nano::election_status &);
	std::shared_ptr<nano::block> get_winner () const;
	nano::amount get_tally () const;
	nano::amount get_final_tally () const;
	std::chrono::milliseconds get_election_end () const;
	std::chrono::milliseconds get_election_duration () const;
	unsigned get_confirmation_request_count () const;
	unsigned get_block_count () const;
	unsigned get_voter_count () const;
	election_status_type get_election_status_type () const;
	void set_winner (std::shared_ptr<nano::block>);
	void set_tally (nano::amount);
	void set_final_tally (nano::amount);
	void set_election_end (std::chrono::milliseconds);
	void set_election_duration (std::chrono::milliseconds);
	void set_confirmation_request_count (uint32_t);
	void set_block_count (uint32_t);
	void set_voter_count (uint32_t);
	void set_election_status_type (nano::election_status_type);
	rsnano::ElectionStatusHandle * handle;
};

nano::wallet_id random_wallet_id ();
}
