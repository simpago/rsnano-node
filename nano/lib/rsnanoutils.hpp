#pragma once

#include "nano/lib/blocks.hpp"

#include <nano/boost/asio/ip/tcp.hpp>
#include <nano/boost/asio/ip/udp.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/node/messages.hpp>

#include <chrono>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <vector>

namespace nano
{
class message;
}

namespace rsnano
{
rsnano::EndpointDto udp_endpoint_to_dto (boost::asio::ip::udp::endpoint const & ep);
rsnano::EndpointDto endpoint_to_dto (boost::asio::ip::tcp::endpoint const & ep);
boost::asio::ip::tcp::endpoint dto_to_endpoint (rsnano::EndpointDto const & dto);
boost::asio::ip::udp::endpoint dto_to_udp_endpoint (rsnano::EndpointDto const & dto);
std::string convert_dto_to_string (rsnano::StringDto & dto);
std::unique_ptr<nano::message> message_handle_to_message (rsnano::MessageHandle * handle);

class async_runtime
{
public:
	async_runtime (bool multi_threaded);
	async_runtime (async_runtime const &) = delete;
	~async_runtime ();
	void stop ();
	boost::asio::io_context io_ctx;
	rsnano::AsyncRuntimeHandle * handle;
};

void read_block_array_dto (rsnano::BlockArrayDto & dto, std::vector<std::shared_ptr<nano::block>> & list_a);
void read_block_deque (rsnano::BlockArrayDto & dto, std::deque<std::shared_ptr<nano::block>> & list_a);
rsnano::BlockArrayDto to_block_array_dto (std::vector<std::shared_ptr<nano::block>> & list_a);

class AtomicU64Wrapper
{
public:
	AtomicU64Wrapper (uint64_t value_a) :
		handle{ rsnano::rsn_atomic_u64_create (value_a) }
	{
	}
	AtomicU64Wrapper (rsnano::AtomicU64Handle * handle_a) :
		handle{ handle_a }
	{
	}
	AtomicU64Wrapper (AtomicU64Wrapper const &) = delete;
	AtomicU64Wrapper (AtomicU64Wrapper &&) = delete;
	~AtomicU64Wrapper ()
	{
		rsnano::rsn_atomic_u64_destroy (handle);
	}

	uint64_t load ()
	{
		return rsnano::rsn_atomic_u64_load (handle);
	}

	void store (uint64_t value_a)
	{
		return rsnano::rsn_atomic_u64_store (handle, value_a);
	}

	void add (uint64_t value_a)
	{
		return rsnano::rsn_atomic_u64_add (handle, value_a);
	}

	rsnano::AtomicU64Handle * handle;
};

class AtomicBoolWrapper
{
public:
	AtomicBoolWrapper (bool value_a) :
		handle{ rsnano::rsn_atomic_bool_create (value_a) }
	{
	}
	AtomicBoolWrapper (rsnano::AtomicBoolHandle * handle_a) :
		handle{ handle_a }
	{
	}
	AtomicBoolWrapper (AtomicBoolWrapper const &) = delete;
	AtomicBoolWrapper (AtomicBoolWrapper &&) = delete;
	~AtomicBoolWrapper ()
	{
		rsnano::rsn_atomic_bool_destroy (handle);
	}

	bool load ()
	{
		return rsnano::rsn_atomic_bool_load (handle);
	}

	void store (bool value_a)
	{
		return rsnano::rsn_atomic_bool_store (handle, value_a);
	}

	rsnano::AtomicBoolHandle * handle;
};

class RsNanoTimer
{
public:
	RsNanoTimer () :
		handle{ rsnano::rsn_timer_create () }
	{
	}
	~RsNanoTimer ()
	{
		rsnano::rsn_timer_destroy (handle);
	}
	RsNanoTimer (RsNanoTimer const &) = delete;
	RsNanoTimer (RsNanoTimer &&) = delete;
	uint64_t elapsed_ms ()
	{
		return rsnano::rsn_timer_elapsed_ms (handle);
	}
	void restart ()
	{
		rsnano::rsn_timer_restart (handle);
	}
	rsnano::TimerHandle * handle;
};

class block_vec
{
public:
	block_vec () :
		handle{ rsnano::rsn_block_vec_create () }
	{
	}

	block_vec (rsnano::BlockVecHandle * handle_a) :
		handle{ handle_a }
	{
	}
	block_vec (std::vector<std::shared_ptr<nano::block>> const & blocks_a) :
		handle{ rsnano::rsn_block_vec_create () }
	{
		for (const auto & block : blocks_a)
		{
			push_back (*block);
		}
	}
	block_vec (std::deque<std::shared_ptr<nano::block>> const & blocks_a) :
		handle{ rsnano::rsn_block_vec_create () }
	{
		for (const auto & block : blocks_a)
		{
			push_back (*block);
		}
	}

	block_vec (block_vec const &) = delete;

	~block_vec ()
	{
		rsnano::rsn_block_vec_destroy (handle);
	}

	void erase_last (size_t count)
	{
		rsnano::rsn_block_vec_erase_last (handle, count);
	}

	void push_back (nano::block const & block)
	{
		rsnano::rsn_block_vec_push_back (handle, block.get_handle ());
	}

	size_t size () const
	{
		return rsnano::rsn_block_vec_size (handle);
	}

	bool empty () const
	{
		return size () == 0;
	}

	void clear ()
	{
		rsnano::rsn_block_vec_clear (handle);
	}

	std::vector<std::shared_ptr<nano::block>> to_vector () const
	{
		std::vector<std::shared_ptr<nano::block>> result;
		result.reserve (size ());
		for (auto i = 0; i < size (); ++i)
		{
			result.push_back (nano::block_handle_to_block (rsnano::rsn_block_vec_get_block (handle, i)));
		}
		return result;
	}

	rsnano::BlockVecHandle * handle;
};

class block_hash_vec
{
public:
	block_hash_vec ();
	block_hash_vec (rsnano::BlockHashVecHandle * handle_a);
	block_hash_vec (block_hash_vec const &);
	block_hash_vec (block_hash_vec &&) = delete;
	~block_hash_vec ();
	block_hash_vec & operator= (block_hash_vec const & other_a);
	bool empty () const;
	size_t size () const;
	void push_back (nano::block_hash const & hash);
	void clear ();
	void assign (block_hash_vec const & source_a, size_t start, size_t end);
	void truncate (size_t new_size_a);
	rsnano::BlockHashVecHandle * handle;
};

std::chrono::system_clock::time_point time_point_from_nanoseconds (uint64_t nanoseconds);

class instant
{
public:
	instant () :
		handle{ rsnano::rsn_instant_now () }
	{
	}
	instant (instant const &) = delete;
	~instant ()
	{
		rsnano::rsn_instant_destroy (handle);
	}
	std::chrono::milliseconds elapsed () const
	{
		return std::chrono::milliseconds{ rsnano::rsn_instant_elapsed_ms (handle) };
	}
	rsnano::InstantHandle * handle;
};

class account_vec
{
public:
	account_vec ();
	explicit account_vec (rsnano::AccountVecHandle * handle);
	account_vec (std::vector<nano::account> accounts);
	account_vec (std::deque<nano::account> accounts);
	~account_vec ();
	void push (nano::account const & account);
	std::size_t size () const;
	std::vector<nano::account> into_vector () const;
	rsnano::AccountVecHandle * handle;
};

class string_vec
{
public:
	string_vec ();
	string_vec (std::vector<std::string> const & values);
	string_vec (string_vec const &) = delete;
	~string_vec ();
	void push (std::string const & value);
	rsnano::StringVecHandle * handle;
};

}
