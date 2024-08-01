#include "nano/lib/rsnano.hpp"
#include "nano/node/messages.hpp"

#include <nano/lib/rsnanoutils.hpp>

#include <chrono>

rsnano::EndpointDto to_endpoint_dto (boost::asio::ip::address const & addr, unsigned short port)
{
	rsnano::EndpointDto dto;
	dto.port = port;
	dto.v6 = addr.is_v6 ();
	if (dto.v6)
	{
		auto bytes{ addr.to_v6 ().to_bytes () };
		std::copy (std::begin (bytes), std::end (bytes), std::begin (dto.bytes));
	}
	else
	{
		auto bytes{ addr.to_v4 ().to_bytes () };
		std::copy (std::begin (bytes), std::end (bytes), std::begin (dto.bytes));
	}
	return dto;
}

rsnano::EndpointDto rsnano::udp_endpoint_to_dto (boost::asio::ip::udp::endpoint const & ep)
{
	return to_endpoint_dto (ep.address (), ep.port ());
}

rsnano::EndpointDto rsnano::endpoint_to_dto (boost::asio::ip::tcp::endpoint const & ep)
{
	return to_endpoint_dto (ep.address (), ep.port ());
}

boost::asio::ip::address dto_to_ip_address (rsnano::EndpointDto const & dto)
{
	if (dto.v6)
	{
		std::array<unsigned char, 16> bytes;
		std::copy (std::begin (dto.bytes), std::end (dto.bytes), std::begin (bytes));
		boost::asio::ip::address_v6 addr_v6{ bytes };
		return boost::asio::ip::address{ addr_v6 };
	}
	std::array<unsigned char, 4> bytes;
	std::copy (dto.bytes, dto.bytes + 4, std::begin (bytes));
	boost::asio::ip::address_v4 addr_v4{ bytes };
	return boost::asio::ip::address{ addr_v4 };
}

boost::asio::ip::udp::endpoint rsnano::dto_to_udp_endpoint (rsnano::EndpointDto const & dto)
{
	return boost::asio::ip::udp::endpoint (dto_to_ip_address (dto), dto.port);
}

boost::asio::ip::tcp::endpoint rsnano::dto_to_endpoint (rsnano::EndpointDto const & dto)
{
	return boost::asio::ip::tcp::endpoint (dto_to_ip_address (dto), dto.port);
}

std::string rsnano::convert_dto_to_string (rsnano::StringDto & dto)
{
	std::string result (dto.value);
	rsnano::rsn_string_destroy (dto.handle);
	return result;
}

rsnano::async_runtime::async_runtime (bool multi_threaded) :
	io_ctx{},
	handle{ rsnano::rsn_async_runtime_create (multi_threaded) }
{
}

rsnano::async_runtime::~async_runtime ()
{
	rsnano::rsn_async_runtime_destroy (handle);
}

void rsnano::async_runtime::stop ()
{
	io_ctx.stop ();
}

std::unique_ptr<nano::message> rsnano::message_handle_to_message (rsnano::MessageHandle * handle)
{
	auto type = static_cast<nano::message_type> (rsnano::rsn_message_type (handle));
	switch (type)
	{
		case nano::message_type::keepalive:
			return std::make_unique<nano::keepalive> (handle);
		case nano::message_type::publish:
			return std::make_unique<nano::publish> (handle);
		case nano::message_type::confirm_req:
			return std::make_unique<nano::confirm_req> (handle);
		case nano::message_type::confirm_ack:
			return std::make_unique<nano::confirm_ack> (handle);
		case nano::message_type::bulk_pull:
			return std::make_unique<nano::bulk_pull> (handle);
		case nano::message_type::bulk_push:
			return std::make_unique<nano::bulk_push> (handle);
		case nano::message_type::frontier_req:
			return std::make_unique<nano::frontier_req> (handle);
		case nano::message_type::node_id_handshake:
			return std::make_unique<nano::node_id_handshake> (handle);
		case nano::message_type::bulk_pull_account:
			return std::make_unique<nano::bulk_pull_account> (handle);
		case nano::message_type::telemetry_req:
			return std::make_unique<nano::telemetry_req> (handle);
		case nano::message_type::telemetry_ack:
			return std::make_unique<nano::telemetry_ack> (handle);
		case nano::message_type::asc_pull_req:
			return std::make_unique<nano::asc_pull_req> (handle);
		case nano::message_type::asc_pull_ack:
			return std::make_unique<nano::asc_pull_ack> (handle);
		default:
			throw std::runtime_error ("invalid message type");
	}
}

void rsnano::read_block_array_dto (rsnano::BlockArrayDto & dto, std::vector<std::shared_ptr<nano::block>> & list_a)
{
	for (int i = 0; i < dto.count; ++i)
	{
		list_a.push_back (nano::block_handle_to_block (dto.blocks[i]));
	}
	rsnano::rsn_block_array_destroy (&dto);
}

void rsnano::read_block_deque (rsnano::BlockArrayDto & dto, std::deque<std::shared_ptr<nano::block>> & list_a)
{
	for (int i = 0; i < dto.count; ++i)
	{
		list_a.push_back (nano::block_handle_to_block (dto.blocks[i]));
	}
	rsnano::rsn_block_array_destroy (&dto);
}

rsnano::block_hash_vec::block_hash_vec () :
	handle{ rsnano::rsn_block_hash_vec_create () }
{
}

rsnano::block_hash_vec::block_hash_vec (rsnano::BlockHashVecHandle * handle_a) :
	handle{ handle_a }
{
}

rsnano::block_hash_vec::block_hash_vec (rsnano::block_hash_vec const & other_a) :
	handle{ rsnano::rsn_block_hash_vec_clone (other_a.handle) }
{
}

rsnano::block_hash_vec::~block_hash_vec ()
{
	rsnano::rsn_block_hash_vec_destroy (handle);
}

rsnano::block_hash_vec & rsnano::block_hash_vec::operator= (rsnano::block_hash_vec const & other_a)
{
	rsnano::rsn_block_hash_vec_destroy (handle);
	handle = rsnano::rsn_block_hash_vec_clone (other_a.handle);
	return *this;
}
bool rsnano::block_hash_vec::empty () const
{
	return size () == 0;
}
size_t rsnano::block_hash_vec::size () const
{
	return rsnano::rsn_block_hash_vec_size (handle);
}
void rsnano::block_hash_vec::push_back (const nano::block_hash & hash)
{
	rsnano::rsn_block_hash_vec_push (handle, hash.bytes.data ());
}
void rsnano::block_hash_vec::clear ()
{
	rsnano::rsn_block_hash_vec_clear (handle);
}
void rsnano::block_hash_vec::assign (block_hash_vec const & source_a, size_t start, size_t end)
{
	rsnano::rsn_block_hash_vec_assign_range (handle, source_a.handle, start, end);
}
void rsnano::block_hash_vec::truncate (size_t new_size_a)
{
	rsnano::rsn_block_hash_vec_truncate (handle, new_size_a);
}

std::chrono::system_clock::time_point rsnano::time_point_from_nanoseconds (uint64_t nanoseconds)
{
	std::chrono::nanoseconds result_ns{ nanoseconds };
	return std::chrono::system_clock::time_point (std::chrono::duration_cast<std::chrono::system_clock::duration> (result_ns));
}

rsnano::account_vec::account_vec () :
	handle{ rsnano::rsn_account_vec_create () }
{
}

rsnano::account_vec::account_vec (rsnano::AccountVecHandle * handle) :
	handle{ handle }
{
}

rsnano::account_vec::~account_vec ()
{
	rsnano::rsn_account_vec_destroy (handle);
}

rsnano::account_vec::account_vec (std::vector<nano::account> accounts) :
	handle{ rsnano::rsn_account_vec_create () }
{
	for (const auto & i : accounts)
	{
		push (i);
	}
}

rsnano::account_vec::account_vec (std::deque<nano::account> accounts) :
	handle{ rsnano::rsn_account_vec_create () }
{
	for (const auto & i : accounts)
	{
		push (i);
	}
}

std::size_t rsnano::account_vec::size () const
{
	return rsnano::rsn_account_vec_len (handle);
}

void rsnano::account_vec::push (nano::account const & account)
{
	rsnano::rsn_account_vec_push (handle, account.bytes.data ());
}

std::vector<nano::account> rsnano::account_vec::into_vector () const
{
	std::vector<nano::account> result;
	result.reserve (size ());
	for (auto i = 0; i < size (); ++i)
	{
		nano::account account;
		rsnano::rsn_account_vec_get (handle, i, account.bytes.data ());
		result.push_back (account);
	}
	return result;
}

rsnano::string_vec::string_vec () :
	handle{ rsnano::rsn_string_vec_create () }
{
}
rsnano::string_vec::string_vec (std::vector<std::string> const & values) :
	handle{ rsnano::rsn_string_vec_create () }
{
	for (const auto & v : values)
	{
		push (v);
	}
}

rsnano::string_vec::~string_vec ()
{
	rsnano::rsn_string_vec_destroy (handle);
}
void rsnano::string_vec::push (std::string const & value)
{
	rsnano::rsn_string_vec_push (handle, value.c_str ());
}
