#pragma once

#include "nano/lib/rsnano.hpp"

#include <nano/lib/asio.hpp>
#include <nano/lib/config.hpp>
#include <nano/lib/errors.hpp>
#include <nano/lib/jsonconfig.hpp>
#include <nano/lib/logging.hpp>
#include <nano/lib/memory.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/stats_enums.hpp>
#include <nano/lib/stream.hpp>
#include <nano/node/common.hpp>
#include <nano/secure/common.hpp>

#include <cstdint>
#include <memory>
#include <string>
#include <variant>
#include <vector>

namespace nano
{
/**
 * Message types are serialized to the network and existing values must thus never change as
 * types are added, removed and reordered in the enum.
 */
enum class message_type : uint8_t
{
	invalid = 0x0,
	not_a_type = 0x1,
	keepalive = 0x2,
	publish = 0x3,
	confirm_req = 0x4,
	confirm_ack = 0x5,
	bulk_pull = 0x6,
	bulk_push = 0x7,
	frontier_req = 0x8,
	/* deleted 0x9 */
	node_id_handshake = 0x0a,
	bulk_pull_account = 0x0b,
	telemetry_req = 0x0c,
	telemetry_ack = 0x0d,
	asc_pull_req = 0x0e,
	asc_pull_ack = 0x0f,
};

enum class bulk_pull_account_flags : uint8_t
{
	pending_hash_and_amount = 0x0,
	pending_address_only = 0x1,
	pending_hash_amount_and_address = 0x2
};

class message_visitor;
class message
{
public:
	explicit message (rsnano::MessageHandle * handle);
	message (message const &) = delete;
	message (message &&) = delete;
	virtual ~message ();
	message & operator= (message const &) = delete;
	message & operator= (message &&) = delete;

	virtual void visit (nano::message_visitor &) const = 0;
	nano::message_type type () const;
	rsnano::MessageHandle * handle;
};

std::unique_ptr<nano::message> message_handle_to_message (rsnano::MessageHandle * handle_a);

class network_constants;

class keepalive final : public message
{
public:
	explicit keepalive (nano::network_constants const & constants);
	keepalive (rsnano::MessageHandle * handle_a);
	keepalive (keepalive const & other_a);
	void visit (nano::message_visitor &) const override;
	bool operator== (nano::keepalive const &) const;
	std::array<nano::endpoint, 8> get_peers () const;
	void set_peers (std::array<nano::endpoint, 8> const & peers_a);
	static std::size_t size ();
	std::string to_string () const;
};

class publish final : public message
{
public:
	publish (nano::network_constants const & constants, std::shared_ptr<nano::block> const &);
	publish (nano::publish const & other_a);
	publish (rsnano::MessageHandle * handle_a);
	void visit (nano::message_visitor &) const override;
	bool operator== (nano::publish const &) const;
	std::shared_ptr<nano::block> get_block () const;
	nano::uint128_t get_digest () const;
	void set_digest (nano::uint128_t digest_a);
	std::string to_string () const;
};

class confirm_req final : public message
{
public:
	confirm_req (nano::network_constants const & constants, std::vector<std::pair<nano::block_hash, nano::root>> const &);
	confirm_req (nano::network_constants const & constants, nano::block_hash const &, nano::root const &);
	confirm_req (rsnano::MessageHandle * handle_a);
	confirm_req (nano::confirm_req const & other_a);
	void visit (nano::message_visitor &) const override;
	bool operator== (nano::confirm_req const &) const;
	std::string roots_string () const;
	std::vector<std::pair<nano::block_hash, nano::root>> get_roots_hashes () const;
	std::string to_string () const;
};

class confirm_ack final : public message
{
public:
	confirm_ack (nano::network_constants const & constants, std::shared_ptr<nano::vote> const &, bool rebroadcasted = false);
	confirm_ack (nano::confirm_ack const & other_a);
	confirm_ack (rsnano::MessageHandle * handle_a);
	void visit (nano::message_visitor &) const override;
	bool operator== (nano::confirm_ack const &) const;
	std::shared_ptr<nano::vote> get_vote () const;
	std::string to_string () const;
};

class frontier_req final : public message
{
public:
	class frontier_req_payload
	{
	public:
		rsnano::FrontierReqPayloadDto to_dto () const;

		nano::account start{};
		uint32_t age{ 0 };
		uint32_t count{ 0 };
		bool only_confirmed{ false };
	};

	frontier_req (nano::network_constants const & constants, frontier_req_payload & payload);
	frontier_req (rsnano::MessageHandle * handle_a);
	frontier_req (frontier_req const &);
	void visit (nano::message_visitor &) const override;
	bool operator== (nano::frontier_req const &) const;
	bool is_only_confirmed_present () const;
	static std::size_t size ();
	nano::account get_start () const;
	uint32_t get_age () const;
	uint32_t get_count () const;
	std::string to_string () const;
};

enum class telemetry_maker : uint8_t
{
	nf_node = 0,
	nf_pruned_node = 1,
	nano_node_light = 2,
	rs_nano_node = 3
};

class telemetry_data
{
public:
	telemetry_data ();
	telemetry_data (nano::telemetry_data const & other_a);
	telemetry_data (nano::telemetry_data && other_a);
	telemetry_data (rsnano::TelemetryDataHandle * handle);
	~telemetry_data ();
	nano::telemetry_data & operator= (nano::telemetry_data const & other_a);

	nano::signature get_signature () const;
	void set_signature (nano::signature const & signature_a);
	nano::account get_node_id () const;
	void set_node_id (nano::account const & node_id_a);
	uint64_t get_block_count () const;
	void set_block_count (uint64_t count_a);
	uint64_t get_cemented_count () const;
	void set_cemented_count (uint64_t count_a);
	uint64_t get_unchecked_count () const;
	void set_unchecked_count (uint64_t count_a);
	uint64_t get_account_count () const;
	void set_account_count (uint64_t count_a);
	uint64_t get_bandwidth_cap () const;
	void set_bandwidth_cap (uint64_t cap_a);
	uint64_t get_uptime () const;
	void set_uptime (uint64_t uptime_a);
	uint32_t get_peer_count () const;
	void set_peer_count (uint32_t count_a);
	uint8_t get_protocol_version () const;
	void set_protocol_version (uint8_t version_a);
	nano::block_hash get_genesis_block () const;
	void set_genesis_block (nano::block_hash const & block_a);
	uint8_t get_major_version () const;
	void set_major_version (uint8_t version_a);
	uint8_t get_minor_version () const;
	void set_minor_version (uint8_t version_a);
	uint8_t get_patch_version () const;
	void set_patch_version (uint8_t version_a);
	uint8_t get_pre_release_version () const;
	void set_pre_release_version (uint8_t version_a);
	uint8_t get_maker () const;
	void set_maker (uint8_t maker_a);
	std::chrono::system_clock::time_point get_timestamp () const;
	void set_timestamp (std::chrono::system_clock::time_point timestamp_a);
	uint64_t get_active_difficulty () const;
	void set_active_difficulty (uint64_t difficulty_a);
	std::vector<uint8_t> get_unknown_data () const;
	void set_unknown_data (std::vector<uint8_t> data_a);

	nano::error serialize_json (nano::jsonconfig &, bool) const;
	nano::error deserialize_json (nano::jsonconfig &, bool);
	void sign (nano::keypair const &);
	bool validate_signature () const;
	bool operator== (nano::telemetry_data const &) const;
	bool operator!= (nano::telemetry_data const &) const;
	std::string to_string () const;

	// Size does not include unknown_data
	static std::size_t size ();
	static std::size_t latest_size ()
	{
		return size ();
	}; // This needs to be updated for each new telemetry version
	rsnano::TelemetryDataHandle * handle;
};

class telemetry_req final : public message
{
public:
	explicit telemetry_req (nano::network_constants const & constants);
	telemetry_req (nano::telemetry_req const &);
	telemetry_req (rsnano::MessageHandle * handle_a);
	void visit (nano::message_visitor &) const override;
	std::string to_string () const;
};

class telemetry_ack final : public message
{
public:
	explicit telemetry_ack (nano::network_constants const & constants);
	telemetry_ack (nano::network_constants const & constants, telemetry_data const &);
	telemetry_ack (nano::telemetry_ack const &);
	telemetry_ack (rsnano::MessageHandle * handle_a);
	telemetry_ack & operator= (telemetry_ack const & other_a);
	void visit (nano::message_visitor &) const override;
	bool is_empty_payload () const;
	std::string to_string () const;
	nano::telemetry_data get_data () const;
};

class bulk_pull final : public message
{
public:
	using count_t = uint32_t;
	class bulk_pull_payload
	{
	public:
		rsnano::BulkPullPayloadDto to_dto () const;

		nano::hash_or_account start{};
		nano::block_hash end{};
		count_t count{ 0 };
		bool ascending{ false };
	};

	bulk_pull (nano::network_constants const & constants, bulk_pull_payload & payload);
	bulk_pull (rsnano::MessageHandle * handle_a);
	bulk_pull (bulk_pull const & other_a);
	void visit (nano::message_visitor &) const override;
	nano::block_hash get_end () const;
	std::string to_string () const;
};

class bulk_pull_account final : public message
{
public:
	class payload
	{
	public:
		nano::account account{};
		nano::amount minimum_amount{};
		bulk_pull_account_flags flags{};
		rsnano::BulkPullAccountPayloadDto to_dto () const;
	};

	bulk_pull_account (nano::network_constants const & constants, bulk_pull_account::payload const & payload);
	bulk_pull_account (rsnano::MessageHandle * handle_a);
	bulk_pull_account (bulk_pull_account const & other_a);
	void visit (nano::message_visitor &) const override;
	static std::size_t size ();
	nano::account get_account () const;
	nano::amount get_minimum_amount () const;
	bulk_pull_account_flags get_flags () const;
	void set_account (nano::account account_a);
	void set_minimum_amount (nano::amount amount_a);
	void set_flags (bulk_pull_account_flags flags_a);
	std::string to_string () const;
};

class bulk_push final : public message
{
public:
	explicit bulk_push (nano::network_constants const & constants);
	bulk_push (rsnano::MessageHandle * handle_a);
	void visit (nano::message_visitor &) const override;
};

class node_id_handshake final : public message
{
public: // Payload definitions
	class query_payload
	{
	public:
		nano::uint256_union cookie;
	};

	class response_payload
	{
	public:
		struct v2_payload
		{
			nano::uint256_union salt;
			nano::block_hash genesis;
		};

		nano::account node_id;
		nano::signature signature;
		std::optional<v2_payload> v2;
	};

public:
	explicit node_id_handshake (nano::network_constants const &, std::optional<query_payload> query = std::nullopt, std::optional<response_payload> response = std::nullopt);
	node_id_handshake (node_id_handshake const &);
	node_id_handshake (rsnano::MessageHandle * handle_a);

	void visit (nano::message_visitor &) const override;
	std::optional<query_payload> get_query () const;
	std::optional<response_payload> get_response () const;
	std::string to_string () const;
	bool is_v2 () const;
};

/**
 * Type of requested asc pull data
 * - blocks:
 * - account_info:
 */
enum class asc_pull_type : uint8_t
{
	invalid = 0x0,
	blocks = 0x1,
	account_info = 0x2,
	frontiers = 0x3,
};

struct empty_payload
{
};

/**
 * Ascending bootstrap pull request
 */
class asc_pull_req final : public message
{
public: // Payload definitions
	enum class hash_type : uint8_t
	{
		account = 0,
		block = 1,
	};

	struct blocks_payload
	{
		nano::hash_or_account start{ 0 };
		uint8_t count{ 0 };
		hash_type start_type{};
	};

	struct account_info_payload
	{
		nano::hash_or_account target{ 0 };
		hash_type target_type{};
	};

	struct frontiers_payload
	{
		nano::account start{ 0 };
		uint16_t count{ 0 };
	};

public:
	using id_t = uint64_t;

	asc_pull_req (nano::network_constants const &, uint64_t id, account_info_payload & payload_a);
	asc_pull_req (nano::network_constants const &, uint64_t id, blocks_payload & payload_a);
	asc_pull_req (nano::network_constants const &, uint64_t id, frontiers_payload & payload_a);
	asc_pull_req (rsnano::MessageHandle * handle_a);
	asc_pull_req (asc_pull_req const & other_a);

	uint64_t id () const;
	void set_id (uint64_t id_a);
	nano::asc_pull_type pull_type () const;

	void visit (nano::message_visitor &) const override;

	std::variant<empty_payload, blocks_payload, account_info_payload, frontiers_payload> payload () const;
};

/**
 * Ascending bootstrap pull response
 */
class asc_pull_ack final : public message
{
public: // Payload definitions
	struct blocks_payload
	{
		/* Header allows for 16 bit extensions; 65535 bytes / 500 bytes (block size with some future margin) ~ 131 */
		constexpr static std::size_t max_blocks = 128;

		std::vector<std::shared_ptr<nano::block>> blocks{};
	};

	struct account_info_payload
	{
		nano::account account{ 0 };
		nano::block_hash account_open{ 0 };
		nano::block_hash account_head{ 0 };
		uint64_t account_block_count{ 0 };
		nano::block_hash account_conf_frontier{ 0 };
		uint64_t account_conf_height{ 0 };
	};

	struct frontiers_payload
	{
		/* Header allows for 16 bit extensions; 65536 bytes / 64 bytes (account + frontier) ~ 1024, but we need some space for null frontier terminator */
		constexpr static std::size_t max_frontiers = 1000;
		using frontier = std::pair<nano::account, nano::block_hash>;

		// Payload
		std::vector<frontier> frontiers;
	};

public:
	using id_t = asc_pull_req::id_t;

	asc_pull_ack (nano::network_constants const &, uint64_t id, account_info_payload & payload_a);
	asc_pull_ack (nano::network_constants const &, uint64_t id, blocks_payload & payload_a);
	asc_pull_ack (nano::network_constants const &, uint64_t id, frontiers_payload & payload_a);
	asc_pull_ack (rsnano::MessageHandle * handle_a);
	asc_pull_ack (asc_pull_ack const & other_a);

	uint64_t id () const;
	void set_id (uint64_t id_a);
	nano::asc_pull_type pull_type () const;

	void visit (nano::message_visitor &) const override;
	std::variant<empty_payload, blocks_payload, account_info_payload, frontiers_payload> payload () const;
};

class message_visitor
{
public:
	virtual void keepalive (nano::keepalive const & message)
	{
		default_handler (message);
	};
	virtual void publish (nano::publish const & message)
	{
		default_handler (message);
	}
	virtual void confirm_req (nano::confirm_req const & message)
	{
		default_handler (message);
	}
	virtual void confirm_ack (nano::confirm_ack const & message)
	{
		default_handler (message);
	}
	virtual void bulk_pull (nano::bulk_pull const & message)
	{
		default_handler (message);
	}
	virtual void bulk_pull_account (nano::bulk_pull_account const & message)
	{
		default_handler (message);
	}
	virtual void bulk_push (nano::bulk_push const & message)
	{
		default_handler (message);
	}
	virtual void frontier_req (nano::frontier_req const & message)
	{
		default_handler (message);
	}
	virtual void node_id_handshake (nano::node_id_handshake const & message)
	{
		default_handler (message);
	}
	virtual void telemetry_req (nano::telemetry_req const & message)
	{
		default_handler (message);
	}
	virtual void telemetry_ack (nano::telemetry_ack const & message)
	{
		default_handler (message);
	}
	virtual void asc_pull_req (nano::asc_pull_req const & message)
	{
		default_handler (message);
	}
	virtual void asc_pull_ack (nano::asc_pull_ack const & message)
	{
		default_handler (message);
	}
	virtual void default_handler (nano::message const &) {};
	virtual ~message_visitor () {};
};

}
