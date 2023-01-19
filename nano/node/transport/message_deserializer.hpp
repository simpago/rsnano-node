#pragma once

#include <nano/node/common.hpp>
#include <nano/node/messages.hpp>
#include <nano/node/socket.hpp>

#include <memory>
#include <vector>

namespace nano
{
class socket;

namespace transport
{
	class message_deserializer : public std::enable_shared_from_this<nano::transport::message_deserializer>
	{
	public:
		enum class parse_status : uint8_t
		{
			none,
			success,
			insufficient_work,
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
			invalid_network,
			outdated_version,
			duplicate_publish_message,
			message_size_too_big,
		};

		using callback_type = std::function<void (boost::system::error_code, std::unique_ptr<nano::message>)>;

		message_deserializer (network_constants const &, network_filter &, block_uniquer &, vote_uniquer &);
		message_deserializer (message_deserializer const &) = delete;
		message_deserializer (message_deserializer &&) = delete;
		~message_deserializer ();

		parse_status get_status () const;

		/*
		 * Asynchronously read next message from socket.
		 * If an irrecoverable error is encountered callback will be called with an error code set and null message.
		 * If a 'soft' error is encountered (eg. duplicate block publish) error won't be set but message will be null. In that case, `status` field will be set to code indicating reason for failure.
		 * If message is received successfully, error code won't be set and message will be non-null. `status` field will be set to `success`.
		 * Should not be called until the previous invocation finishes and calls the callback.
		 */
		void read (std::shared_ptr<nano::socket> socket, callback_type const && callback);

	private: // Dependencies
		rsnano::MessageDeserializerHandle * handle_m;

		static stat::detail to_stat_detail (parse_status);
		static std::string to_string (parse_status);
	};

}
}
