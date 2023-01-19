#include "nano/lib/rsnano.hpp"
#include "nano/lib/rsnanoutils.hpp"

#include <nano/lib/blocks.hpp>
#include <nano/lib/memory.hpp>
#include <nano/lib/work.hpp>
#include <nano/node/active_transactions.hpp>
#include <nano/node/common.hpp>
#include <nano/node/election.hpp>
#include <nano/node/messages.hpp>
#include <nano/node/network.hpp>
#include <nano/node/wallet.hpp>
#include <nano/secure/buffer.hpp>

#include <boost/endian/conversion.hpp>
#include <boost/format.hpp>
#include <boost/pool/pool_alloc.hpp>

#include <sstream>

/*
 * message_header
 */

nano::message_header::message_header (nano::message_header const & other_a) :
	handle{ rsnano::rsn_message_header_clone (other_a.handle) }
{
}

nano::message_header::message_header (nano::message_header && other_a) :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::message_header::message_header (rsnano::MessageHeaderHandle * handle_a) :
	handle{ handle_a }
{
}

nano::message_header::message_header (bool & error_a, nano::stream & stream_a) :
	handle{ rsnano::rsn_message_header_empty () }
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}

nano::message_header & nano::message_header::operator= (nano::message_header && other_a)
{
	if (handle != nullptr)
		rsnano::rsn_message_header_destroy (handle);
	handle = other_a.handle;
	other_a.handle = nullptr;
	return *this;
}

nano::message_header & nano::message_header::operator= (message_header const & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_message_header_destroy (handle);
	handle = rsnano::rsn_message_header_clone (other_a.handle);
	return *this;
}

nano::message_header::~message_header ()
{
	if (handle != nullptr)
		rsnano::rsn_message_header_destroy (handle);
}

void nano::message_header::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_header_serialize (handle, &stream_a))
	{
		throw new std::runtime_error ("could not serialize message header");
	}
}

bool nano::message_header::deserialize (nano::stream & stream_a)
{
	auto error = !rsnano::rsn_message_header_deserialize (handle, &stream_a);
	return error;
}

nano::block_type nano::message_header::block_type () const
{
	return static_cast<nano::block_type> (rsnano::rsn_message_header_block_type (handle));
}

void nano::message_header::flag_set (uint8_t flag_a)
{
	// Flags from 8 are block_type & count
	debug_assert (flag_a < 8);
	set_extension (flag_a, true);
}

nano::networks nano::message_header::get_network () const
{
	return static_cast<nano::networks> (rsnano::rsn_message_header_network (handle));
}

void nano::message_header::set_network (nano::networks network)
{
	rsnano::rsn_message_header_set_network (handle, static_cast<uint16_t> (network));
}

uint8_t nano::message_header::get_version_using () const
{
	return rsnano::rsn_message_header_version_using (handle);
}

void nano::message_header::set_version_using (uint8_t version_a)
{
	rsnano::rsn_message_header_set_version_using (handle, version_a);
}

nano::message_type nano::message_header::get_type () const
{
	return static_cast<nano::message_type> (rsnano::rsn_message_header_type (handle));
}

nano::stat::detail nano::to_stat_detail (nano::message_type message_type)
{
	return static_cast<nano::stat::detail> (rsnano::rsn_message_type_to_stat_detail (static_cast<uint8_t> (message_type)));
}

void nano::message_header::set_extension (std::size_t position, bool value)
{
	rsnano::rsn_message_header_set_extension (handle, position, value);
}

size_t nano::message_header::size ()
{
	return rsnano::rsn_message_header_size ();
}

/*
 * message
 */

nano::message::message (rsnano::MessageHandle * handle_a) :
	handle (handle_a)
{
}

nano::message::~message ()
{
	rsnano::rsn_message_destroy (handle);
}

std::shared_ptr<std::vector<uint8_t>> nano::message::to_bytes () const
{
	auto bytes = std::make_shared<std::vector<uint8_t>> ();
	nano::vectorstream stream (*bytes);
	serialize (stream);
	return bytes;
}

nano::shared_const_buffer nano::message::to_shared_const_buffer () const
{
	return shared_const_buffer (to_bytes ());
}

nano::message_header nano::message::get_header () const
{
	return nano::message_header{ rsnano::rsn_message_header (handle) };
}

void nano::message::set_header (nano::message_header const & header_a)
{
	rsnano::rsn_message_set_header (handle, header_a.handle);
}

nano::message_type nano::message::type () const
{
	return get_header ().get_type ();
}

/*
 * message_parser
 */

// MTU - IP header - UDP header
std::size_t const nano::message_parser::max_safe_udp_message_size = 508;

std::string nano::message_parser::status_string ()
{
	switch (status)
	{
		case nano::message_parser::parse_status::success:
		{
			return "success";
		}
		case nano::message_parser::parse_status::insufficient_work:
		{
			return "insufficient_work";
		}
		case nano::message_parser::parse_status::invalid_header:
		{
			return "invalid_header";
		}
		case nano::message_parser::parse_status::invalid_message_type:
		{
			return "invalid_message_type";
		}
		case nano::message_parser::parse_status::invalid_keepalive_message:
		{
			return "invalid_keepalive_message";
		}
		case nano::message_parser::parse_status::invalid_publish_message:
		{
			return "invalid_publish_message";
		}
		case nano::message_parser::parse_status::invalid_confirm_req_message:
		{
			return "invalid_confirm_req_message";
		}
		case nano::message_parser::parse_status::invalid_confirm_ack_message:
		{
			return "invalid_confirm_ack_message";
		}
		case nano::message_parser::parse_status::invalid_node_id_handshake_message:
		{
			return "invalid_node_id_handshake_message";
		}
		case nano::message_parser::parse_status::invalid_telemetry_req_message:
		{
			return "invalid_telemetry_req_message";
		}
		case nano::message_parser::parse_status::invalid_telemetry_ack_message:
		{
			return "invalid_telemetry_ack_message";
		}
		case nano::message_parser::parse_status::outdated_version:
		{
			return "outdated_version";
		}
		case nano::message_parser::parse_status::duplicate_publish_message:
		{
			return "duplicate_publish_message";
		}
	}

	debug_assert (false);

	return "[unknown parse_status]";
}

std::unique_ptr<nano::message> nano::message_handle_to_message (rsnano::MessageHandle * handle_a)
{
	if (handle_a == nullptr)
		return nullptr;

	auto msg_type{ static_cast<nano::message_type> (rsnano::rsn_message_type (handle_a)) };
	std::unique_ptr<nano::message> result;
	switch (msg_type)
	{
		case nano::message_type::bulk_pull:
			result = std::make_unique<nano::bulk_pull> (handle_a);
			break;
		case nano::message_type::keepalive:
			result = std::make_unique<nano::keepalive> (handle_a);
			break;
		case nano::message_type::publish:
			result = std::make_unique<nano::publish> (handle_a);
			break;
		case nano::message_type::confirm_req:
			result = std::make_unique<nano::confirm_req> (handle_a);
			break;
		case nano::message_type::confirm_ack:
			result = std::make_unique<nano::confirm_ack> (handle_a);
			break;
		case nano::message_type::bulk_push:
			result = std::make_unique<nano::bulk_push> (handle_a);
			break;
		case nano::message_type::frontier_req:
			result = std::make_unique<nano::frontier_req> (handle_a);
			break;
		case nano::message_type::node_id_handshake:
			result = std::make_unique<nano::node_id_handshake> (handle_a);
			break;
		case nano::message_type::bulk_pull_account:
			result = std::make_unique<nano::bulk_pull_account> (handle_a);
			break;
		case nano::message_type::telemetry_req:
			result = std::make_unique<nano::telemetry_req> (handle_a);
			break;
		case nano::message_type::telemetry_ack:
			result = std::make_unique<nano::telemetry_ack> (handle_a);
			break;
		default:
			break;
	}
	return result;
}

nano::message_parser::message_parser (nano::network_filter & publish_filter_a, nano::block_uniquer & block_uniquer_a, nano::vote_uniquer & vote_uniquer_a, nano::message_visitor & visitor_a, nano::work_pool & pool_a, nano::network_constants const & network) :
	publish_filter (publish_filter_a),
	block_uniquer (block_uniquer_a),
	vote_uniquer (vote_uniquer_a),
	visitor (visitor_a),
	pool (pool_a),
	status (parse_status::success),
	network{ network }
{
}

void nano::message_parser::deserialize_buffer (uint8_t const * buffer_a, std::size_t size_a)
{
	status = parse_status::success;
	auto error (false);
	if (size_a <= max_safe_udp_message_size)
	{
		// Guaranteed to be deliverable
		nano::bufferstream stream (buffer_a, size_a);
		nano::message_header header (error, stream);
		if (!error)
		{
			if (header.get_network () != network.current_network)
			{
				status = parse_status::invalid_header;
				return;
			}

			if (header.get_version_using () < network.protocol_version_min)
			{
				status = parse_status::outdated_version;
			}
			else
			{
				switch (header.get_type ())
				{
					case nano::message_type::keepalive:
					{
						deserialize_keepalive (stream, header);
						break;
					}
					case nano::message_type::publish:
					{
						nano::uint128_t digest;
						if (!publish_filter.apply (buffer_a + message_header::size (), size_a - message_header::size (), &digest))
						{
							deserialize_publish (stream, header, digest);
						}
						else
						{
							status = parse_status::duplicate_publish_message;
						}
						break;
					}
					case nano::message_type::confirm_req:
					{
						deserialize_confirm_req (stream, header);
						break;
					}
					case nano::message_type::confirm_ack:
					{
						deserialize_confirm_ack (stream, header);
						break;
					}
					case nano::message_type::node_id_handshake:
					{
						deserialize_node_id_handshake (stream, header);
						break;
					}
					case nano::message_type::telemetry_req:
					{
						deserialize_telemetry_req (stream, header);
						break;
					}
					case nano::message_type::telemetry_ack:
					{
						deserialize_telemetry_ack (stream, header);
						break;
					}
					default:
					{
						status = parse_status::invalid_message_type;
						break;
					}
				}
			}
		}
		else
		{
			status = parse_status::invalid_header;
		}
	}
}

void nano::message_parser::deserialize_keepalive (nano::stream & stream_a, nano::message_header const & header_a)
{
	auto error (false);
	nano::keepalive incoming (error, stream_a, header_a);
	if (!error && at_end (stream_a))
	{
		visitor.keepalive (incoming);
	}
	else
	{
		status = parse_status::invalid_keepalive_message;
	}
}

void nano::message_parser::deserialize_publish (nano::stream & stream_a, nano::message_header const & header_a, nano::uint128_t const & digest_a)
{
	auto error (false);
	nano::publish incoming (error, stream_a, header_a, digest_a, &block_uniquer);
	if (!error && at_end (stream_a))
	{
		auto block{ incoming.get_block () };
		if (!network.work.validate_entry (*block))
		{
			visitor.publish (incoming);
		}
		else
		{
			status = parse_status::insufficient_work;
		}
	}
	else
	{
		status = parse_status::invalid_publish_message;
	}
}

void nano::message_parser::deserialize_confirm_req (nano::stream & stream_a, nano::message_header const & header_a)
{
	auto error (false);
	nano::confirm_req incoming (error, stream_a, header_a, &block_uniquer);
	if (!error && at_end (stream_a))
	{
		if (incoming.get_block () == nullptr || !network.work.validate_entry (*incoming.get_block ()))
		{
			visitor.confirm_req (incoming);
		}
		else
		{
			status = parse_status::insufficient_work;
		}
	}
	else
	{
		status = parse_status::invalid_confirm_req_message;
	}
}

void nano::message_parser::deserialize_confirm_ack (nano::stream & stream_a, nano::message_header const & header_a)
{
	auto error (false);
	nano::confirm_ack incoming (error, stream_a, header_a, &vote_uniquer);
	if (!error && at_end (stream_a))
	{
		visitor.confirm_ack (incoming);
	}
	else
	{
		status = parse_status::invalid_confirm_ack_message;
	}
}

void nano::message_parser::deserialize_node_id_handshake (nano::stream & stream_a, nano::message_header const & header_a)
{
	bool error_l (false);
	nano::node_id_handshake incoming (error_l, stream_a, header_a);
	if (!error_l && at_end (stream_a))
	{
		visitor.node_id_handshake (incoming);
	}
	else
	{
		status = parse_status::invalid_node_id_handshake_message;
	}
}

void nano::message_parser::deserialize_telemetry_req (nano::stream & stream_a, nano::message_header const & header_a)
{
	nano::telemetry_req incoming (header_a);
	if (at_end (stream_a))
	{
		visitor.telemetry_req (incoming);
	}
	else
	{
		status = parse_status::invalid_telemetry_req_message;
	}
}

void nano::message_parser::deserialize_telemetry_ack (nano::stream & stream_a, nano::message_header const & header_a)
{
	bool error_l (false);
	nano::telemetry_ack incoming (error_l, stream_a, header_a);
	// Intentionally not checking if at the end of stream, because these messages support backwards/forwards compatibility
	if (!error_l)
	{
		visitor.telemetry_ack (incoming);
	}
	else
	{
		status = parse_status::invalid_telemetry_ack_message;
	}
}

bool nano::message_parser::at_end (nano::stream & stream_a)
{
	uint8_t junk;
	auto end (nano::try_read (stream_a, junk));
	return end;
}

/*
 * keepalive
 */

rsnano::MessageHandle * create_keepalive_handle (nano::network_constants const & constants, int16_t version_using)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_keepalive_create (&constants_dto, version_using);
}

nano::keepalive::keepalive (nano::network_constants const & constants) :
	message (create_keepalive_handle (constants, -1))
{
}

nano::keepalive::keepalive (nano::network_constants const & constants, uint8_t version_using_a) :
	message (create_keepalive_handle (constants, version_using_a))
{
}

nano::keepalive::keepalive (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

nano::keepalive::keepalive (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a) :
	message (rsnano::rsn_message_keepalive_create2 (header_a.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}

nano::keepalive::keepalive (keepalive const & other_a) :
	message (rsnano::rsn_message_keepalive_clone (other_a.handle))
{
}

void nano::keepalive::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.keepalive (*this);
}

void nano::keepalive::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_keepalive_serialize (handle, &stream_a))
	{
		throw std::runtime_error ("could not serialize keepalive");
	}
}

bool nano::keepalive::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_keepalive_deserialize (handle, &stream_a);
	return error;
}

bool nano::keepalive::operator== (nano::keepalive const & other_a) const
{
	return get_peers () == other_a.get_peers ();
}

std::array<nano::endpoint, 8> nano::keepalive::get_peers () const
{
	rsnano::EndpointDto dtos[8];
	rsnano::rsn_message_keepalive_peers (handle, &dtos[0]);
	std::array<nano::endpoint, 8> result;
	for (auto i = 0; i < 8; ++i)
	{
		result[i] = rsnano::dto_to_udp_endpoint (dtos[i]);
	}
	return result;
}

void nano::keepalive::set_peers (std::array<nano::endpoint, 8> const & peers_a)
{
	rsnano::EndpointDto dtos[8];
	for (auto i = 0; i < 8; ++i)
	{
		dtos[i] = rsnano::udp_endpoint_to_dto (peers_a[i]);
	}
	rsnano::rsn_message_keepalive_set_peers (handle, dtos);
}

std::size_t nano::keepalive::size ()
{
	return rsnano::rsn_message_keepalive_size ();
}

std::string nano::keepalive::to_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_keepalive_to_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

/*
 * publish
 */

rsnano::MessageHandle * create_publish_handle2 (nano::message_header const & header_a, nano::uint128_t const & digest_a)
{
	uint8_t bytes[16] = { 0 };
	boost::multiprecision::export_bits (digest_a, std::rbegin (bytes), 8, false);
	return rsnano::rsn_message_publish_create2 (header_a.handle, &bytes[0]);
}

nano::publish::publish (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a, nano::uint128_t const & digest_a, nano::block_uniquer * uniquer_a) :
	message (create_publish_handle2 (header_a, digest_a))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a, uniquer_a);
	}
}

rsnano::MessageHandle * create_publish_handle (nano::network_constants const & constants, std::shared_ptr<nano::block> const & block_a)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_publish_create (&constants_dto, block_a->get_handle ());
}

nano::publish::publish (nano::network_constants const & constants, std::shared_ptr<nano::block> const & block_a) :
	message (create_publish_handle (constants, block_a))
{
}

nano::publish::publish (nano::publish const & other_a) :
	message (rsnano::rsn_message_publish_clone (other_a.handle))
{
}

nano::publish::publish (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

void nano::publish::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_publish_serialize (handle, &stream_a))
	{
		throw std::runtime_error ("could not serialize publish message");
	}
}

bool nano::publish::deserialize (nano::stream & stream_a, nano::block_uniquer * uniquer_a)
{
	rsnano::BlockUniquerHandle * uniquer_handle = nullptr;
	if (uniquer_a != nullptr)
	{
		uniquer_handle = uniquer_a->handle;
	}
	bool error = !rsnano::rsn_message_publish_deserialize (handle, &stream_a, uniquer_handle);
	return error;
}

void nano::publish::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.publish (*this);
}

bool nano::publish::operator== (nano::publish const & other_a) const
{
	return *get_block () == *other_a.get_block ();
}

std::shared_ptr<nano::block> nano::publish::get_block () const
{
	auto block_handle = rsnano::rsn_message_publish_block (handle);
	if (block_handle == nullptr)
		return nullptr;
	return nano::block_handle_to_block (block_handle);
}

nano::uint128_t nano::publish::get_digest () const
{
	std::uint8_t bytes[16];
	rsnano::rsn_message_publish_digest (handle, &bytes[0]);
	nano::uint128_t result;
	boost::multiprecision::import_bits (result, std::begin (bytes), std::end (bytes));
	return result;
}

void nano::publish::set_digest (nano::uint128_t digest_a)
{
	std::uint8_t bytes[16];
	boost::multiprecision::export_bits (digest_a, std::rbegin (bytes), 8, false);
	rsnano::rsn_message_publish_set_digest (handle, &bytes[0]);
}

std::string nano::publish::to_string () const
{
	rsnano::StringDto string_dto;
	rsnano::rsn_message_publish_to_string (handle, &string_dto);
	return rsnano::convert_dto_to_string (string_dto);
}

/*
 * confirm_req
 */

rsnano::MessageHandle * create_confirm_req_handle (nano::network_constants const & constants, nano::block const * block_a, std::vector<std::pair<nano::block_hash, nano::root>> roots_hashes_a)
{
	auto constants_dto{ constants.to_dto () };
	rsnano::BlockHandle * block_handle = nullptr;
	if (block_a != nullptr)
	{
		block_handle = block_a->get_handle ();
	}

	size_t hashes_count = roots_hashes_a.size ();
	std::vector<rsnano::HashRootPair> dtos;
	dtos.reserve (hashes_count);
	for (const auto & i : roots_hashes_a)
	{
		rsnano::HashRootPair dto;
		std::copy (std::begin (i.first.bytes), std::end (i.first.bytes), std::begin (dto.block_hash));
		std::copy (std::begin (i.second.bytes), std::end (i.second.bytes), std::begin (dto.root));
		dtos.push_back (dto);
	}

	return rsnano::rsn_message_confirm_req_create (&constants_dto, block_handle, dtos.data (), hashes_count);
}

nano::confirm_req::confirm_req (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a, nano::block_uniquer * uniquer_a) :
	message (rsnano::rsn_message_confirm_req_create2 (header_a.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a, uniquer_a);
	}
}

nano::confirm_req::confirm_req (nano::network_constants const & constants, std::shared_ptr<nano::block> const & block_a) :
	message (create_confirm_req_handle (constants, block_a.get (), std::vector<std::pair<nano::block_hash, nano::root>> ()))
{
}

nano::confirm_req::confirm_req (nano::network_constants const & constants, std::vector<std::pair<nano::block_hash, nano::root>> const & roots_hashes_a) :
	message (create_confirm_req_handle (constants, nullptr, roots_hashes_a))
{
}

nano::confirm_req::confirm_req (nano::network_constants const & constants, nano::block_hash const & hash_a, nano::root const & root_a) :
	message (create_confirm_req_handle (constants, nullptr, std::vector<std::pair<nano::block_hash, nano::root>> (1, std::make_pair (hash_a, root_a))))
{
}

nano::confirm_req::confirm_req (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

nano::confirm_req::confirm_req (nano::confirm_req const & other_a) :
	message (rsnano::rsn_message_confirm_req_clone (other_a.handle))
{
}

std::shared_ptr<nano::block> nano::confirm_req::get_block () const
{
	auto block_handle = rsnano::rsn_message_confirm_req_block (handle);
	std::shared_ptr<nano::block> result;
	if (block_handle != nullptr)
	{
		result = nano::block_handle_to_block (block_handle);
	}
	return result;
}

std::vector<std::pair<nano::block_hash, nano::root>> nano::confirm_req::get_roots_hashes () const
{
	auto count = rsnano::rsn_message_confirm_req_roots_hashes_count (handle);
	std::vector<rsnano::HashRootPair> dtos;
	dtos.resize (count);
	rsnano::rsn_message_confirm_req_roots_hashes (handle, dtos.data ());
	std::vector<std::pair<nano::block_hash, nano::root>> result;
	result.reserve (dtos.size ());
	for (const auto & i : dtos)
	{
		nano::block_hash hash;
		nano::root root;
		std::copy (std::begin (i.block_hash), std::end (i.block_hash), std::begin (hash.bytes));
		std::copy (std::begin (i.root), std::end (i.root), std::begin (root.bytes));
		result.emplace_back (hash, root);
	}
	return result;
}

void nano::confirm_req::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.confirm_req (*this);
}

void nano::confirm_req::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_confirm_req_serialize (handle, &stream_a))
	{
		throw std::runtime_error ("could not serialize confirm_req");
	}
}

bool nano::confirm_req::deserialize (nano::stream & stream_a, nano::block_uniquer * uniquer_a)
{
	rsnano::BlockUniquerHandle * uniquer_handle = nullptr;
	if (uniquer_a != nullptr)
	{
		uniquer_handle = uniquer_a->handle;
	}

	bool error = !rsnano::rsn_message_confirm_req_deserialize (handle, &stream_a, uniquer_handle);
	return error;
}

bool nano::confirm_req::operator== (nano::confirm_req const & other_a) const
{
	return rsnano::rsn_message_confirm_req_equals (handle, other_a.handle);
}

std::string nano::confirm_req::roots_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_confirm_req_roots_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

std::size_t nano::confirm_req::size (nano::block_type type_a, std::size_t count)
{
	return rsnano::rsn_message_confirm_req_size (static_cast<uint8_t> (type_a), count);
}

std::string nano::confirm_req::to_string () const
{
	rsnano::StringDto string_dto;
	rsnano::rsn_message_confirm_req_to_string (handle, &string_dto);
	return rsnano::convert_dto_to_string (string_dto);
}

/*
 * confirm_ack
 */

rsnano::MessageHandle * create_confirm_ack_handle (nano::network_constants const & constants, nano::vote const & vote_a)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_confirm_ack_create (&constants_dto, vote_a.get_handle ());
}

rsnano::MessageHandle * create_confirm_ack_handle (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a, nano::vote_uniquer * uniquer_a)
{
	rsnano::VoteUniquerHandle * uniquer_handle = nullptr;
	if (uniquer_a != nullptr)
	{
		uniquer_handle = uniquer_a->handle;
	}
	return rsnano::rsn_message_confirm_ack_create2 (header_a.handle, &stream_a, uniquer_handle, &error_a);
}

nano::confirm_ack::confirm_ack (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a, nano::vote_uniquer * uniquer_a) :
	message (create_confirm_ack_handle (error_a, stream_a, header_a, uniquer_a))
{
}

nano::confirm_ack::confirm_ack (nano::network_constants const & constants, std::shared_ptr<nano::vote> const & vote_a) :
	message (create_confirm_ack_handle (constants, *vote_a))
{
}

nano::confirm_ack::confirm_ack (nano::confirm_ack const & other_a) :
	message (rsnano::rsn_message_confirm_ack_clone (other_a.handle))
{
}

nano::confirm_ack::confirm_ack (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

void nano::confirm_ack::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_confirm_ack_serialize (handle, &stream_a))
	{
		throw std::runtime_error ("could not serialize confirm_ack");
	}
}

bool nano::confirm_ack::operator== (nano::confirm_ack const & other_a) const
{
	auto result (*get_vote () == *other_a.get_vote ());
	return result;
}

void nano::confirm_ack::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.confirm_ack (*this);
}

std::size_t nano::confirm_ack::size (std::size_t count)
{
	return rsnano::rsn_message_confirm_ack_size (count);
}

std::shared_ptr<nano::vote> nano::confirm_ack::get_vote () const
{
	auto vote_handle{ rsnano::rsn_message_confirm_ack_vote (handle) };
	std::shared_ptr<nano::vote> result;
	if (vote_handle != nullptr)
	{
		result = std::make_shared<nano::vote> (vote_handle);
	}
	return result;
}

std::string nano::confirm_ack::to_string () const
{
	rsnano::StringDto string_dto;
	rsnano::rsn_message_confirm_ack_to_string (handle, &string_dto);
	return rsnano::convert_dto_to_string (string_dto);
}

/*
 * frontier_req
 */

rsnano::MessageHandle * create_frontier_req_handle (nano::network_constants const & constants)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_frontier_req_create (&constants_dto);
}

nano::frontier_req::frontier_req (nano::network_constants const & constants) :
	message (create_frontier_req_handle (constants))
{
}

nano::frontier_req::frontier_req (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a) :
	message (rsnano::rsn_message_frontier_req_create2 (header_a.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}

nano::frontier_req::frontier_req (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

nano::frontier_req::frontier_req (frontier_req const & other_a) :
	message (rsnano::rsn_message_frontier_req_clone (other_a.handle))
{
}

void nano::frontier_req::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_frontier_req_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize frontier_req");
}

bool nano::frontier_req::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_frontier_req_deserialize (handle, &stream_a);
	return error;
}

void nano::frontier_req::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.frontier_req (*this);
}

bool nano::frontier_req::operator== (nano::frontier_req const & other_a) const
{
	return get_start () == other_a.get_start () && get_age () == other_a.get_age () && get_count () == other_a.get_count ();
}

bool nano::frontier_req::is_only_confirmed_present () const
{
	return rsnano::rsn_message_frontier_req_is_confirmed_present (handle);
}

nano::account nano::frontier_req::get_start () const
{
	nano::account start;
	rsnano::rsn_message_frontier_req_start (handle, start.bytes.data ());
	return start;
}

uint32_t nano::frontier_req::get_age () const
{
	return rsnano::rsn_message_frontier_req_age (handle);
}

uint32_t nano::frontier_req::get_count () const
{
	return rsnano::rsn_message_frontier_req_count (handle);
}

void nano::frontier_req::set_start (nano::account const & account)
{
	rsnano::rsn_message_frontier_req_set_start (handle, account.bytes.data ());
}

void nano::frontier_req::set_age (uint32_t age_a)
{
	rsnano::rsn_message_frontier_req_set_age (handle, age_a);
}

void nano::frontier_req::set_count (uint32_t count_a)
{
	rsnano::rsn_message_frontier_req_set_count (handle, count_a);
}

std::size_t nano::frontier_req::size ()
{
	return rsnano::rsn_message_frontier_size ();
}

std::string nano::frontier_req::to_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_frontier_req_to_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

/*
 * bulk_pull
 */

rsnano::MessageHandle * create_bulk_pull_handle (nano::network_constants const & constants)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_bulk_pull_create (&constants_dto);
}

nano::bulk_pull::bulk_pull (nano::network_constants const & constants) :
	message (create_bulk_pull_handle (constants))
{
}

nano::bulk_pull::bulk_pull (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a) :
	message (rsnano::rsn_message_bulk_pull_create2 (header_a.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}

nano::bulk_pull::bulk_pull (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

nano::bulk_pull::bulk_pull (bulk_pull const & other_a) :
	message (rsnano::rsn_message_bulk_pull_req_clone (other_a.handle))
{
}

nano::hash_or_account nano::bulk_pull::get_start () const
{
	nano::hash_or_account start;
	rsnano::rsn_message_bulk_pull_start (handle, start.bytes.data ());
	return start;
}

nano::block_hash nano::bulk_pull::get_end () const
{
	nano::block_hash end;
	rsnano::rsn_message_bulk_pull_end (handle, end.bytes.data ());
	return end;
}

uint32_t nano::bulk_pull::get_count () const
{
	return rsnano::rsn_message_bulk_pull_count (handle);
}

void nano::bulk_pull::set_start (nano::hash_or_account start_a)
{
	rsnano::rsn_message_bulk_pull_set_start (handle, start_a.bytes.data ());
}

void nano::bulk_pull::set_end (nano::block_hash end_a)
{
	rsnano::rsn_message_bulk_pull_set_end (handle, end_a.bytes.data ());
}

void nano::bulk_pull::set_count (uint32_t count_a)
{
	rsnano::rsn_message_bulk_pull_set_count (handle, count_a);
}

void nano::bulk_pull::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.bulk_pull (*this);
}

void nano::bulk_pull::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_bulk_pull_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize bulk_pull");
}

bool nano::bulk_pull::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_bulk_pull_deserialize (handle, &stream_a);
	return error;
}

bool nano::bulk_pull::is_count_present () const
{
	return rsnano::rsn_message_bulk_pull_is_count_present (handle);
}

void nano::bulk_pull::set_count_present (bool value_a)
{
	rsnano::rsn_message_bulk_pull_set_count_present (handle, value_a);
}

bool nano::bulk_pull::is_ascending () const
{
	return rsnano::rsn_message_bulk_pull_is_ascending (handle);
}

void nano::bulk_pull::set_ascending ()
{
	rsnano::rsn_message_bulk_pull_set_ascending (handle);
}

std::string nano::bulk_pull::to_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_bulk_pull_to_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

/*
 * bulk_pull_account
 */

rsnano::MessageHandle * create_bulk_pull_account_handle (nano::network_constants const & constants)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_bulk_pull_account_create (&constants_dto);
}

nano::bulk_pull_account::bulk_pull_account (nano::network_constants const & constants) :
	message (create_bulk_pull_account_handle (constants))
{
}

nano::bulk_pull_account::bulk_pull_account (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a) :
	message (rsnano::rsn_message_bulk_pull_account_create2 (header_a.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}

nano::bulk_pull_account::bulk_pull_account (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

nano::bulk_pull_account::bulk_pull_account (bulk_pull_account const & other_a) :
	message (rsnano::rsn_message_bulk_pull_account_clone (other_a.handle))
{
}

void nano::bulk_pull_account::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.bulk_pull_account (*this);
}

std::size_t nano::bulk_pull_account::size ()
{
	return rsnano::rsn_message_bulk_pull_account_size ();
}

nano::account nano::bulk_pull_account::get_account () const
{
	nano::account account;
	rsnano::rsn_message_bulk_pull_account_account (handle, account.bytes.data ());
	return account;
}

nano::amount nano::bulk_pull_account::get_minimum_amount () const
{
	nano::amount amount;
	rsnano::rsn_message_bulk_pull_account_minimum_amount (handle, amount.bytes.data ());
	return amount;
}

nano::bulk_pull_account_flags nano::bulk_pull_account::get_flags () const
{
	return static_cast<nano::bulk_pull_account_flags> (rsnano::rsn_message_bulk_pull_account_flags (handle));
}

void nano::bulk_pull_account::set_account (nano::account account_a)
{
	rsnano::rsn_message_bulk_pull_account_set_account (handle, account_a.bytes.data ());
}

void nano::bulk_pull_account::set_minimum_amount (nano::amount amount_a)
{
	rsnano::rsn_message_bulk_pull_account_set_minimum_amount (handle, amount_a.bytes.data ());
}

void nano::bulk_pull_account::set_flags (nano::bulk_pull_account_flags flags_a)
{
	rsnano::rsn_message_bulk_pull_account_set_flags (handle, static_cast<uint8_t> (flags_a));
}

void nano::bulk_pull_account::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_bulk_pull_account_serialize (handle, &stream_a))
		throw std::runtime_error ("bulk_pull_account could not be serialized");
}

bool nano::bulk_pull_account::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_bulk_pull_account_deserialize (handle, &stream_a);
	return error;
}

std::string nano::bulk_pull_account::to_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_bulk_pull_account_to_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

/*
 * bulk_push
 */

rsnano::MessageHandle * create_bulk_push_handle (nano::network_constants const & constants)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_bulk_push_create (&constants_dto);
}

nano::bulk_push::bulk_push (nano::network_constants const & constants) :
	message (create_bulk_push_handle (constants))
{
}

nano::bulk_push::bulk_push (nano::message_header const & header_a) :
	message (rsnano::rsn_message_bulk_push_create2 (header_a.handle))
{
}

nano::bulk_push::bulk_push (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

bool nano::bulk_push::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_bulk_push_deserialize (handle, &stream_a);
	return error;
}

void nano::bulk_push::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_bulk_push_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize bulk_push");
}

void nano::bulk_push::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.bulk_push (*this);
}

/*
 * telemetry_req
 */

rsnano::MessageHandle * create_telemetry_req_handle (nano::network_constants const & constants)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_telemetry_req_create (&constants_dto);
}

nano::telemetry_req::telemetry_req (nano::network_constants const & constants) :
	message (create_telemetry_req_handle (constants))
{
}

nano::telemetry_req::telemetry_req (nano::message_header const & header_a) :
	message (rsnano::rsn_message_telemetry_req_create2 (header_a.handle))
{
}

nano::telemetry_req::telemetry_req (nano::telemetry_req const & other_a) :
	message (rsnano::rsn_message_telemetry_req_clone (other_a.handle))
{
}

nano::telemetry_req::telemetry_req (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

bool nano::telemetry_req::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_telemetry_req_deserialize (handle, &stream_a);
	return error;
}

void nano::telemetry_req::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_telemetry_req_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize telemetry_req");
}

void nano::telemetry_req::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.telemetry_req (*this);
}

std::string nano::telemetry_req::to_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_telemetry_req_to_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

/*
 * telemetry_ack
 */

rsnano::MessageHandle * create_telemetry_ack_handle (nano::network_constants const & constants, rsnano::TelemetryDataHandle const * data_handle)
{
	auto constants_dto{ constants.to_dto () };
	nano::telemetry_data default_data;
	if (data_handle == nullptr)
	{
		data_handle = default_data.handle;
	}
	return rsnano::rsn_message_telemetry_ack_create (&constants_dto, data_handle);
}

nano::telemetry_ack::telemetry_ack (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

nano::telemetry_ack::telemetry_ack (nano::network_constants const & constants) :
	message (create_telemetry_ack_handle (constants, nullptr))
{
}

nano::telemetry_ack::telemetry_ack (bool & error_a, nano::stream & stream_a, nano::message_header const & message_header) :
	message (rsnano::rsn_message_telemetry_ack_create2 (message_header.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}

nano::telemetry_ack::telemetry_ack (nano::telemetry_ack const & other_a) :
	message (rsnano::rsn_message_telemetry_ack_clone (other_a.handle))
{
}

nano::telemetry_ack::telemetry_ack (nano::network_constants const & constants, nano::telemetry_data const & telemetry_data_a) :
	message (create_telemetry_ack_handle (constants, telemetry_data_a.handle))
{
}

nano::telemetry_ack & nano::telemetry_ack::operator= (telemetry_ack const & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_message_destroy (handle);
	handle = rsnano::rsn_message_telemetry_ack_clone (other_a.handle);
	return *this;
}

void nano::telemetry_ack::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_telemetry_ack_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize telemetry_ack");
}

bool nano::telemetry_ack::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_telemetry_ack_deserialize (handle, &stream_a);
	return error;
}

void nano::telemetry_ack::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.telemetry_ack (*this);
}

uint16_t nano::telemetry_ack::size () const
{
	return rsnano::rsn_message_telemetry_ack_size (handle);
}

uint16_t nano::telemetry_ack::size (nano::message_header const & message_header_a)
{
	return rsnano::rsn_message_telemetry_ack_size_from_header (message_header_a.handle);
}

nano::telemetry_data nano::telemetry_ack::get_data () const
{
	auto data_handle = rsnano::rsn_message_telemetry_ack_data (handle);
	return nano::telemetry_data{ data_handle };
}

bool nano::telemetry_ack::is_empty_payload () const
{
	return rsnano::rsn_message_telemetry_ack_is_empty_payload (handle);
}

std::string nano::telemetry_ack::to_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_telemetry_ack_to_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

/*
 * telemetry_data
 */

nano::telemetry_data::telemetry_data () :
	handle{ rsnano::rsn_telemetry_data_create () }
{
}

nano::telemetry_data::telemetry_data (rsnano::TelemetryDataHandle * handle_a) :
	handle{ handle_a }
{
}

nano::telemetry_data::telemetry_data (nano::telemetry_data const & other_a) :
	handle{ rsnano::rsn_telemetry_data_clone (other_a.handle) }
{
}

nano::telemetry_data::telemetry_data (nano::telemetry_data && other_a) :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::telemetry_data::~telemetry_data ()
{
	if (handle != nullptr)
		rsnano::rsn_telemetry_data_destroy (handle);
}

nano::telemetry_data & nano::telemetry_data::operator= (nano::telemetry_data const & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_telemetry_data_destroy (handle);
	handle = rsnano::rsn_telemetry_data_clone (other_a.handle);
	return *this;
}

nano::signature nano::telemetry_data::get_signature () const
{
	nano::signature result;
	rsnano::rsn_telemetry_data_get_signature (handle, result.bytes.data ());
	return result;
}

void nano::telemetry_data::set_signature (nano::signature const & signature_a)
{
	rsnano::rsn_telemetry_data_set_signature (handle, signature_a.bytes.data ());
}

nano::account nano::telemetry_data::get_node_id () const
{
	nano::account result;
	rsnano::rsn_telemetry_data_get_node_id (handle, result.bytes.data ());
	return result;
}

void nano::telemetry_data::set_node_id (nano::account const & node_id_a)
{
	rsnano::rsn_telemetry_data_set_node_id (handle, node_id_a.bytes.data ());
}

uint64_t nano::telemetry_data::get_block_count () const
{
	return rsnano::rsn_telemetry_data_get_block_count (handle);
}

void nano::telemetry_data::set_block_count (uint64_t count_a)
{
	rsnano::rsn_telemetry_data_set_block_count (handle, count_a);
}

uint64_t nano::telemetry_data::get_cemented_count () const
{
	return rsnano::rsn_telemetry_data_get_cemented_count (handle);
}

void nano::telemetry_data::set_cemented_count (uint64_t count_a)
{
	rsnano::rsn_telemetry_data_set_cemented_count (handle, count_a);
}

uint64_t nano::telemetry_data::get_unchecked_count () const
{
	return rsnano::rsn_telemetry_data_get_unchecked_count (handle);
}

void nano::telemetry_data::set_unchecked_count (uint64_t count_a)
{
	rsnano::rsn_telemetry_data_set_unchecked_count (handle, count_a);
}

uint64_t nano::telemetry_data::get_account_count () const
{
	return rsnano::rsn_telemetry_data_get_account_count (handle);
}

void nano::telemetry_data::set_account_count (uint64_t count_a)
{
	rsnano::rsn_telemetry_data_set_account_count (handle, count_a);
}

uint64_t nano::telemetry_data::get_bandwidth_cap () const
{
	return rsnano::rsn_telemetry_data_get_bandwidth_cap (handle);
}

void nano::telemetry_data::set_bandwidth_cap (uint64_t cap_a)
{
	rsnano::rsn_telemetry_data_set_bandwidth_cap (handle, cap_a);
}

uint64_t nano::telemetry_data::get_uptime () const
{
	return rsnano::rsn_telemetry_data_get_uptime (handle);
}

void nano::telemetry_data::set_uptime (uint64_t uptime_a)
{
	rsnano::rsn_telemetry_data_set_uptime (handle, uptime_a);
}

uint32_t nano::telemetry_data::get_peer_count () const
{
	return rsnano::rsn_telemetry_data_get_peer_count (handle);
}

void nano::telemetry_data::set_peer_count (uint32_t count_a)
{
	rsnano::rsn_telemetry_data_set_peer_count (handle, count_a);
}

uint8_t nano::telemetry_data::get_protocol_version () const
{
	return rsnano::rsn_telemetry_data_get_protocol_version (handle);
}

void nano::telemetry_data::set_protocol_version (uint8_t version_a)
{
	rsnano::rsn_telemetry_data_set_protocol_version (handle, version_a);
}

nano::block_hash nano::telemetry_data::get_genesis_block () const
{
	nano::block_hash result;
	rsnano::rsn_telemetry_data_get_genesis_block (handle, result.bytes.data ());
	return result;
}

void nano::telemetry_data::set_genesis_block (nano::block_hash const & block_a)
{
	rsnano::rsn_telemetry_data_set_genesis_block (handle, block_a.bytes.data ());
}

uint8_t nano::telemetry_data::get_major_version () const
{
	return rsnano::rsn_telemetry_data_get_major_version (handle);
}

void nano::telemetry_data::set_major_version (uint8_t version_a)
{
	rsnano::rsn_telemetry_data_set_major_version (handle, version_a);
}

uint8_t nano::telemetry_data::get_minor_version () const
{
	return rsnano::rsn_telemetry_data_get_minor_version (handle);
}

void nano::telemetry_data::set_minor_version (uint8_t version_a)
{
	rsnano::rsn_telemetry_data_set_minor_version (handle, version_a);
}

uint8_t nano::telemetry_data::get_patch_version () const
{
	return rsnano::rsn_telemetry_data_get_patch_version (handle);
}

void nano::telemetry_data::set_patch_version (uint8_t version_a)
{
	rsnano::rsn_telemetry_data_set_patch_version (handle, version_a);
}

uint8_t nano::telemetry_data::get_pre_release_version () const
{
	return rsnano::rsn_telemetry_data_get_pre_release_version (handle);
}

void nano::telemetry_data::set_pre_release_version (uint8_t version_a)
{
	rsnano::rsn_telemetry_data_set_pre_release_version (handle, version_a);
}

uint8_t nano::telemetry_data::get_maker () const
{
	return rsnano::rsn_telemetry_data_get_maker (handle);
}

void nano::telemetry_data::set_maker (uint8_t maker_a)
{
	rsnano::rsn_telemetry_data_set_maker (handle, maker_a);
}

std::chrono::system_clock::time_point nano::telemetry_data::get_timestamp () const
{
	auto timestamp_ms = rsnano::rsn_telemetry_data_get_timestamp_ms (handle);
	return std::chrono::system_clock::time_point (std::chrono::duration_cast<std::chrono::system_clock::duration> (std::chrono::milliseconds (timestamp_ms)));
}

void nano::telemetry_data::set_timestamp (std::chrono::system_clock::time_point timestamp_a)
{
	rsnano::rsn_telemetry_data_set_timestamp (handle, std::chrono::duration_cast<std::chrono::milliseconds> (timestamp_a.time_since_epoch ()).count ());
}

uint64_t nano::telemetry_data::get_active_difficulty () const
{
	return rsnano::rsn_telemetry_data_get_active_difficulty (handle);
}

void nano::telemetry_data::set_active_difficulty (uint64_t difficulty_a)
{
	rsnano::rsn_telemetry_data_set_active_difficulty (handle, difficulty_a);
}

std::vector<uint8_t> nano::telemetry_data::get_unknown_data () const
{
	std::vector<uint8_t> result;
	result.resize (rsnano::rsn_telemetry_data_get_unknown_data_len (handle));
	rsnano::rsn_telemetry_data_get_unknown_data (handle, result.data ());
	return result;
}

void nano::telemetry_data::set_unknown_data (std::vector<uint8_t> data_a)
{
	rsnano::rsn_telemetry_data_set_unknown_data (handle, data_a.data (), data_a.size ());
}

void nano::telemetry_data::deserialize (nano::stream & stream_a, uint16_t payload_length_a)
{
	if (!rsnano::rsn_telemetry_data_deserialize (handle, &stream_a, payload_length_a))
		throw std::runtime_error ("could not deserialize telemetry data");
}

void nano::telemetry_data::serialize (nano::stream & stream_a) const
{
	rsnano::rsn_telemetry_data_serialize (handle, &stream_a);
}

nano::error nano::telemetry_data::serialize_json (nano::jsonconfig & json, bool ignore_identification_metrics_a) const
{
	json.put ("block_count", get_block_count ());
	json.put ("cemented_count", get_cemented_count ());
	json.put ("unchecked_count", get_unchecked_count ());
	json.put ("account_count", get_account_count ());
	json.put ("bandwidth_cap", get_bandwidth_cap ());
	json.put ("peer_count", get_peer_count ());
	json.put ("protocol_version", get_protocol_version ());
	json.put ("uptime", get_uptime ());
	json.put ("genesis_block", get_genesis_block ().to_string ());
	json.put ("major_version", get_major_version ());
	json.put ("minor_version", get_minor_version ());
	json.put ("patch_version", get_patch_version ());
	json.put ("pre_release_version", get_pre_release_version ());
	json.put ("maker", get_maker ());
	json.put ("timestamp", std::chrono::duration_cast<std::chrono::milliseconds> (get_timestamp ().time_since_epoch ()).count ());
	json.put ("active_difficulty", nano::to_string_hex (get_active_difficulty ()));
	// Keep these last for UI purposes
	if (!ignore_identification_metrics_a)
	{
		json.put ("node_id", get_node_id ().to_node_id ());
		json.put ("signature", get_signature ().to_string ());
	}
	return json.get_error ();
}

nano::error nano::telemetry_data::deserialize_json (nano::jsonconfig & json, bool ignore_identification_metrics_a)
{
	if (!ignore_identification_metrics_a)
	{
		std::string signature_l;
		json.get ("signature", signature_l);
		if (!json.get_error ())
		{
			nano::signature sig;
			if (sig.decode_hex (signature_l))
			{
				json.get_error ().set ("Could not deserialize signature");
			}
			set_signature (sig);
		}

		std::string node_id_l;
		json.get ("node_id", node_id_l);
		if (!json.get_error ())
		{
			nano::account nid;
			if (nid.decode_node_id (node_id_l))
			{
				json.get_error ().set ("Could not deserialize node id");
			}
			set_node_id (nid);
		}
	}

	uint64_t tmp_u64;
	json.get ("block_count", tmp_u64);
	set_block_count (tmp_u64);

	json.get ("cemented_count", tmp_u64);
	set_cemented_count (tmp_u64);

	json.get ("unchecked_count", tmp_u64);
	set_unchecked_count (tmp_u64);

	json.get ("account_count", tmp_u64);
	set_account_count (tmp_u64);

	json.get ("bandwidth_cap", tmp_u64);
	set_bandwidth_cap (tmp_u64);

	uint32_t tmp_u32;
	json.get ("peer_count", tmp_u32);
	set_peer_count (tmp_u32);

	uint8_t tmp_u8;
	json.get ("protocol_version", tmp_u8);
	set_protocol_version (tmp_u8);

	json.get ("uptime", tmp_u64);
	set_uptime (tmp_u64);

	std::string genesis_block_l;
	json.get ("genesis_block", genesis_block_l);
	if (!json.get_error ())
	{
		nano::block_hash blk;
		if (blk.decode_hex (genesis_block_l))
		{
			json.get_error ().set ("Could not deserialize genesis block");
		}
		set_genesis_block (blk);
	}

	json.get ("major_version", tmp_u8);
	set_major_version (tmp_u8);

	json.get ("minor_version", tmp_u8);
	set_minor_version (tmp_u8);

	json.get ("patch_version", tmp_u8);
	set_patch_version (tmp_u8);

	json.get ("pre_release_version", tmp_u8);
	set_pre_release_version (tmp_u8);

	json.get ("maker", tmp_u8);
	set_maker (tmp_u8);

	auto timestamp_l = json.get<uint64_t> ("timestamp");
	auto tsp = std::chrono::system_clock::time_point (std::chrono::milliseconds (timestamp_l));
	set_timestamp (tsp);

	auto current_active_difficulty_text = json.get<std::string> ("active_difficulty");
	auto ec = nano::from_string_hex (current_active_difficulty_text, tmp_u64);
	set_active_difficulty (tmp_u64);
	debug_assert (!ec);
	return json.get_error ();
}

std::string nano::telemetry_data::to_string () const
{
	rsnano::StringDto string_dto;
	rsnano::rsn_telemetry_data_to_json (handle, &string_dto);
	return rsnano::convert_dto_to_string (string_dto);
}

bool nano::telemetry_data::operator== (nano::telemetry_data const & data_a) const
{
	return (get_signature () == data_a.get_signature () && get_node_id () == data_a.get_node_id () && get_block_count () == data_a.get_block_count ()
	&& get_cemented_count () == data_a.get_cemented_count () && get_unchecked_count () == data_a.get_unchecked_count () && get_account_count () == data_a.get_account_count ()
	&& get_bandwidth_cap () == data_a.get_bandwidth_cap () && get_uptime () == data_a.get_uptime ()
	&& get_peer_count () == data_a.get_peer_count () && get_protocol_version () == data_a.get_protocol_version () && get_genesis_block () == data_a.get_genesis_block ()
	&& get_major_version () == data_a.get_major_version () && get_minor_version () == data_a.get_minor_version () && get_patch_version () == data_a.get_patch_version ()
	&& get_pre_release_version () == data_a.get_pre_release_version () && get_maker () == data_a.get_maker () && get_timestamp () == data_a.get_timestamp ()
	&& get_active_difficulty () == data_a.get_active_difficulty () && get_unknown_data () == data_a.get_unknown_data ());
}

bool nano::telemetry_data::operator!= (nano::telemetry_data const & data_a) const
{
	return !(*this == data_a);
}

void nano::telemetry_data::sign (nano::keypair const & node_id_a)
{
	if (!rsnano::rsn_telemetry_data_sign (handle, node_id_a.prv.bytes.data ()))
		throw std::runtime_error ("could not sign telemetry data");
}

bool nano::telemetry_data::validate_signature () const
{
	bool error = !rsnano::rsn_telemetry_data_validate_signature (handle);
	return error;
}

std::size_t nano::telemetry_data::size ()
{
	return rsnano::rsn_telemetry_data_size ();
}

/*
 * node_id_handshake
 */

rsnano::MessageHandle * create_node_id_handshake_handle (nano::network_constants const & constants, boost::optional<nano::uint256_union> query, boost::optional<std::pair<nano::account, nano::signature>> response)
{
	auto constants_dto{ constants.to_dto () };
	const uint8_t * query_bytes = nullptr;
	if (query)
	{
		query_bytes = query->bytes.data ();
	}

	const uint8_t * acc_bytes = nullptr;
	const uint8_t * sig_bytes = nullptr;
	if (response)
	{
		acc_bytes = response->first.bytes.data ();
		sig_bytes = response->second.bytes.data ();
	}

	return rsnano::rsn_message_node_id_handshake_create (&constants_dto, query_bytes, acc_bytes, sig_bytes);
}

nano::node_id_handshake::node_id_handshake (bool & error_a, nano::stream & stream_a, nano::message_header const & header_a) :
	message (rsnano::rsn_message_node_id_handshake_create2 (header_a.handle))
{
	error_a = deserialize (stream_a);
}

nano::node_id_handshake::node_id_handshake (node_id_handshake const & other_a) :
	message{ rsnano::rsn_message_node_id_handshake_clone (other_a.handle) }
{
}

nano::node_id_handshake::node_id_handshake (nano::network_constants const & constants, boost::optional<nano::uint256_union> query, boost::optional<std::pair<nano::account, nano::signature>> response) :
	message (create_node_id_handshake_handle (constants, query, response))
{
}

nano::node_id_handshake::node_id_handshake (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

void nano::node_id_handshake::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_node_id_handshake_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize node_id_handshake");
}

bool nano::node_id_handshake::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_node_id_handshake_deserialize (handle, &stream_a);
	return error;
}

boost::optional<nano::uint256_union> nano::node_id_handshake::get_query () const
{
	nano::uint256_union data;
	if (rsnano::rsn_message_node_id_handshake_query (handle, data.bytes.data ()))
		return boost::optional<nano::uint256_union> (data);

	return boost::none;
}

boost::optional<std::pair<nano::account, nano::signature>> nano::node_id_handshake::get_response () const
{
	nano::account account;
	nano::signature signature;
	if (rsnano::rsn_message_node_id_handshake_response (handle, account.bytes.data (), signature.bytes.data ()))
	{
		return boost::optional<std::pair<nano::account, nano::signature>> (std::make_pair (account, signature));
	}
	return boost::none;
}

bool nano::node_id_handshake::operator== (nano::node_id_handshake const & other_a) const
{
	auto result (*get_query () == *other_a.get_query () && *get_response () == *other_a.get_response ());
	return result;
}

void nano::node_id_handshake::visit (nano::message_visitor & visitor_a) const
{
	visitor_a.node_id_handshake (*this);
}

std::size_t nano::node_id_handshake::size () const
{
	return size (get_header ());
}

std::size_t nano::node_id_handshake::size (nano::message_header const & header_a)
{
	return rsnano::rsn_message_node_id_handshake_size (header_a.handle);
}

std::string nano::node_id_handshake::to_string () const
{
	rsnano::StringDto dto;
	rsnano::rsn_message_node_id_handshake_to_string (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

/*
 * asc_pull_req
 */
rsnano::MessageHandle * create_asc_pull_req_handle (nano::network_constants const & constants)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_asc_pull_req_create (&constants_dto);
}

nano::asc_pull_req::asc_pull_req (const nano::network_constants & constants) :
	message (create_asc_pull_req_handle (constants))
{
}

nano::asc_pull_req::asc_pull_req (bool & error_a, nano::stream & stream_a, const nano::message_header & header_a) :
	message (rsnano::rsn_message_asc_pull_req_create2 (header_a.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}
nano::asc_pull_req::asc_pull_req (nano::asc_pull_req const & other_a) :
	message{ rsnano::rsn_message_asc_pull_req_clone (other_a.handle) }
{
}

nano::asc_pull_req::asc_pull_req (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

uint64_t nano::asc_pull_req::id () const
{
	return rsnano::rsn_message_asc_pull_req_get_id (handle);
}

void nano::asc_pull_req::set_id (uint64_t id_a)
{
	rsnano::rsn_message_asc_pull_req_set_id (handle, id_a);
}

nano::asc_pull_type nano::asc_pull_req::pull_type () const
{
	return static_cast<nano::asc_pull_type> (rsnano::rsn_message_asc_pull_req_pull_type (handle));
}

void nano::asc_pull_req::visit (nano::message_visitor & visitor) const
{
	visitor.asc_pull_req (*this);
}

void nano::asc_pull_req::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_message_asc_pull_req_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize asc_pull_req");
}

bool nano::asc_pull_req::deserialize (nano::stream & stream_a)
{
	bool error = !rsnano::rsn_message_asc_pull_req_deserialize (handle, &stream_a);
	return error;
}

std::size_t nano::asc_pull_req::size (const nano::message_header & header)
{
	return rsnano::rsn_message_asc_pull_req_size (header.handle);
}

void nano::asc_pull_req::request_blocks (blocks_payload & payload_a)
{
	rsnano::rsn_message_asc_pull_req_request_blocks (handle, payload_a.start.bytes.data (), payload_a.count, static_cast<uint8_t> (payload_a.start_type));
}

void nano::asc_pull_req::request_account_info (account_info_payload & payload_a)
{
	rsnano::rsn_message_asc_pull_req_request_account_info (handle, payload_a.target.bytes.data (), static_cast<uint8_t> (payload_a.target_type));
}

void nano::asc_pull_req::request_invalid ()
{
	rsnano::rsn_message_asc_pull_req_request_invalid (handle);
}

std::variant<nano::empty_payload, nano::asc_pull_req::blocks_payload, nano::asc_pull_req::account_info_payload> nano::asc_pull_req::payload () const
{
	std::variant<nano::empty_payload, nano::asc_pull_req::blocks_payload, nano::asc_pull_req::account_info_payload> result;
	auto payload_type = static_cast<nano::asc_pull_type> (rsnano::rsn_message_asc_pull_req_payload_type (handle));
	if (payload_type == nano::asc_pull_type::blocks)
	{
		nano::asc_pull_req::blocks_payload blocks;
		uint8_t start_type{ 0 };
		rsnano::rsn_message_asc_pull_req_payload_blocks (handle, blocks.start.bytes.data (), &blocks.count, &start_type);
		blocks.start_type = static_cast<nano::asc_pull_req::hash_type> (start_type);
		return blocks;
	}
	else if (payload_type == nano::asc_pull_type::account_info)
	{
		nano::asc_pull_req::account_info_payload account_info;
		uint8_t target_type{ 0 };
		rsnano::rsn_message_asc_pull_req_payload_account_info (handle, account_info.target.bytes.data (), &target_type);
		account_info.target_type = static_cast<nano::asc_pull_req::hash_type> (target_type);
		return account_info;
	}
	return empty_payload{};
}

/*
 * asc_pull_ack
 */
rsnano::MessageHandle * create_asc_pull_ack_handle (nano::network_constants const & constants)
{
	auto constants_dto{ constants.to_dto () };
	return rsnano::rsn_message_asc_pull_ack_create (&constants_dto);
}

nano::asc_pull_ack::asc_pull_ack (const nano::network_constants & constants) :
	message (create_asc_pull_ack_handle (constants))
{
}

nano::asc_pull_ack::asc_pull_ack (bool & error_a, nano::stream & stream_a, const nano::message_header & header_a) :
	message (rsnano::rsn_message_asc_pull_ack_create2 (header_a.handle))
{
	if (!error_a)
	{
		error_a = deserialize (stream_a);
	}
}
nano::asc_pull_ack::asc_pull_ack (nano::asc_pull_ack const & other_a) :
	message{ rsnano::rsn_message_asc_pull_ack_clone (other_a.handle) }
{
}

nano::asc_pull_ack::asc_pull_ack (rsnano::MessageHandle * handle_a) :
	message (handle_a)
{
}

uint64_t nano::asc_pull_ack::id () const
{
	return rsnano::rsn_message_asc_pull_ack_get_id (handle);
}

void nano::asc_pull_ack::set_id (uint64_t id_a)
{
	rsnano::rsn_message_asc_pull_ack_set_id (handle, id_a);
}

nano::asc_pull_type nano::asc_pull_ack::pull_type () const
{
	return static_cast<nano::asc_pull_type> (rsnano::rsn_message_asc_pull_ack_pull_type (handle));
}

void nano::asc_pull_ack::visit (nano::message_visitor & visitor) const
{
	visitor.asc_pull_ack (*this);
}

void nano::asc_pull_ack::serialize (nano::stream & stream) const
{
	if (!rsnano::rsn_message_asc_pull_ack_serialize (handle, &stream))
		throw std::runtime_error ("could not serialize asc_pull_ack");
}

bool nano::asc_pull_ack::deserialize (nano::stream & stream)
{
	bool error = !rsnano::rsn_message_asc_pull_ack_deserialize (handle, &stream);
	return error;
}

std::size_t nano::asc_pull_ack::size (const nano::message_header & header)
{
	return rsnano::rsn_message_asc_pull_ack_size (header.handle);
}

void nano::asc_pull_ack::request_blocks (blocks_payload & payload_a)
{
	std::vector<rsnano::BlockHandle *> block_handles;
	block_handles.reserve (payload_a.blocks.size ());
	for (const auto & block : payload_a.blocks)
	{
		block_handles.push_back (block->get_handle ());
	}
	rsnano::rsn_message_asc_pull_ack_request_blocks (handle, block_handles.data (), block_handles.size ());
}

void nano::asc_pull_ack::request_account_info (account_info_payload & payload_a)
{
	rsnano::AccountInfoAckPayloadDto dto;
	std::copy (std::begin (payload_a.account.bytes), std::end (payload_a.account.bytes), std::begin (dto.account));
	std::copy (std::begin (payload_a.account_open.bytes), std::end (payload_a.account_open.bytes), std::begin (dto.account_open));
	std::copy (std::begin (payload_a.account_head.bytes), std::end (payload_a.account_head.bytes), std::begin (dto.account_head));
	dto.account_block_count = payload_a.account_block_count;
	std::copy (std::begin (payload_a.account_conf_frontier.bytes), std::end (payload_a.account_conf_frontier.bytes), std::begin (dto.account_conf_frontier));
	dto.account_conf_height = payload_a.account_conf_height;
	rsnano::rsn_message_asc_pull_ack_request_account_info (handle, &dto);
}

void nano::asc_pull_ack::request_invalid ()
{
	rsnano::rsn_message_asc_pull_ack_request_invalid (handle);
}

std::variant<nano::empty_payload, nano::asc_pull_ack::blocks_payload, nano::asc_pull_ack::account_info_payload> nano::asc_pull_ack::payload () const
{
	std::variant<nano::empty_payload, nano::asc_pull_req::blocks_payload, nano::asc_pull_req::account_info_payload> result;
	auto payload_type = static_cast<nano::asc_pull_type> (rsnano::rsn_message_asc_pull_ack_payload_type (handle));
	if (payload_type == nano::asc_pull_type::blocks)
	{
		nano::asc_pull_ack::blocks_payload blocks;
		rsnano::BlockArrayDto blocks_dto;
		rsnano::rsn_message_asc_pull_ack_payload_blocks (handle, &blocks_dto);
		rsnano::read_block_array_dto (blocks_dto, blocks.blocks);
		return blocks;
	}
	else if (payload_type == nano::asc_pull_type::account_info)
	{
		rsnano::AccountInfoAckPayloadDto dto;
		rsnano::rsn_message_asc_pull_ack_payload_account_info (handle, &dto);
		nano::asc_pull_ack::account_info_payload account_info;
		std::copy (std::begin (dto.account), std::end (dto.account), std::begin (account_info.account.bytes));
		std::copy (std::begin (dto.account_open), std::end (dto.account_open), std::begin (account_info.account_open.bytes));
		std::copy (std::begin (dto.account_head), std::end (dto.account_head), std::begin (account_info.account_head.bytes));
		account_info.account_block_count = dto.account_block_count;
		std::copy (std::begin (dto.account_conf_frontier), std::end (dto.account_conf_frontier), std::begin (account_info.account_conf_frontier.bytes));
		account_info.account_conf_height = dto.account_conf_height;
		return account_info;
	}
	return empty_payload{};
}
