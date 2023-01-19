#include "nano/lib/rsnanoutils.hpp"

#include <nano/lib/stats.hpp>
#include <nano/node/confirmation_height_unbounded.hpp>
#include <nano/node/logging.hpp>
#include <nano/node/write_database_queue.hpp>
#include <nano/secure/ledger.hpp>

#include <boost/format.hpp>

#include <numeric>

namespace
{
void notify_observers_callback_wrapper (void * context, rsnano::BlockHandle * const * block_handles, size_t len)
{
	auto fn = static_cast<std::function<void (std::vector<std::shared_ptr<nano::block>> const &)> *> (context);
	std::vector<std::shared_ptr<nano::block>> blocks;
	for (int i = 0; i < len; ++i)
	{
		blocks.push_back (nano::block_handle_to_block (rsnano::rsn_block_clone (block_handles[i])));
	}

	(*fn) (blocks);
}

void drop_notify_observers_callback (void * context)
{
	auto fn = static_cast<std::function<void (std::vector<std::shared_ptr<nano::block>> const &)> *> (context);
	delete fn;
}

void notify_block_already_cemented_callback_wrapper (void * context, const uint8_t * block_hash_a)
{
	auto fn = static_cast<std::function<void (nano::block_hash const &)> *> (context);
	nano::block_hash hash;
	std::copy (block_hash_a, block_hash_a + 32, std::begin (hash.bytes));
	(*fn) (hash);
}

void drop_notify_block_already_cemented_callback (void * context_a)
{
	auto fn = static_cast<std::function<void (nano::block_hash const &)> *> (context_a);
	delete fn;
}

uint64_t awaiting_processing_size_callback_wrapper (void * context_a)
{
	auto fn = static_cast<std::function<uint64_t ()> *> (context_a);
	return (*fn) ();
}

void drop_awaiting_processing_size_callback (void * context_a)
{
	auto fn = static_cast<std::function<uint64_t ()> *> (context_a);
	delete fn;
}
}

nano::confirmation_height_unbounded::confirmation_height_unbounded (nano::ledger & ledger_a, nano::stat & stats_a, nano::write_database_queue & write_database_queue_a, std::chrono::milliseconds batch_separate_pending_min_time_a, nano::logging const & logging_a, std::shared_ptr<nano::logger_mt> & logger_a, rsnano::AtomicU64Wrapper & batch_write_size_a, std::function<void (std::vector<std::shared_ptr<nano::block>> const &)> const & notify_observers_callback_a, std::function<void (nano::block_hash const &)> const & notify_block_already_cemented_observers_callback_a, std::function<uint64_t ()> const & awaiting_processing_size_callback_a)
{
	auto logging_dto{ logging_a.to_dto () };
	handle = rsnano::rsn_conf_height_unbounded_create (
	ledger_a.handle,
	nano::to_logger_handle (logger_a),
	&logging_dto,
	stats_a.handle,
	static_cast<uint64_t> (batch_separate_pending_min_time_a.count ()),
	batch_write_size_a.handle,
	write_database_queue_a.handle,
	notify_observers_callback_wrapper,
	new std::function<void (std::vector<std::shared_ptr<nano::block>> const &)>{ notify_observers_callback_a },
	drop_notify_observers_callback,
	notify_block_already_cemented_callback_wrapper,
	new std::function<void (nano::block_hash const &)>{ notify_block_already_cemented_observers_callback_a },
	drop_notify_block_already_cemented_callback,
	awaiting_processing_size_callback_wrapper,
	new std::function<uint64_t ()>{ awaiting_processing_size_callback_a },
	drop_awaiting_processing_size_callback);
}

nano::confirmation_height_unbounded::~confirmation_height_unbounded ()
{
	rsnano::rsn_conf_height_unbounded_destroy (handle);
}

void nano::confirmation_height_unbounded::process (std::shared_ptr<nano::block> original_block)
{
	rsnano::rsn_conf_height_unbounded_process (handle, original_block->get_handle ());
}

void nano::confirmation_height_unbounded::cement_blocks ()
{
	rsnano::rsn_conf_height_unbounded_cement_blocks (handle);
}

bool nano::confirmation_height_unbounded::pending_empty () const
{
	return rsnano::rsn_conf_height_unbounded_pending_empty (handle);
}

size_t nano::confirmation_height_unbounded::pending_writes_size () const
{
	return rsnano::rsn_conf_height_unbounded_pending_writes_size_safe (handle);
}

void nano::confirmation_height_unbounded::clear_process_vars ()
{
	rsnano::rsn_conf_height_unbounded_clear_process_vars (handle);
}

bool nano::confirmation_height_unbounded::has_iterated_over_block (nano::block_hash const & hash_a) const
{
	return rsnano::rsn_conf_height_unbounded_has_iterated_over_block (handle, hash_a.bytes.data ());
}

void nano::confirmation_height_unbounded::stop ()
{
	rsnano::rsn_conf_height_unbounded_stop (handle);
}

uint64_t nano::confirmation_height_unbounded::block_cache_size () const
{
	return rsnano::rsn_conf_height_unbounded_block_cache_size (handle);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (confirmation_height_unbounded & confirmation_height_unbounded, std::string const & name_a)
{
	auto composite = std::make_unique<container_info_composite> (name_a);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "confirmed_iterated_pairs", rsnano::rsn_conf_height_unbounded_conf_iterated_pairs_len (confirmation_height_unbounded.handle), rsnano::rsn_conf_iterated_pair_size () }));
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "pending_writes", rsnano::rsn_conf_height_unbounded_pending_writes_len (confirmation_height_unbounded.handle), rsnano::rsn_conf_height_details_size () }));
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "implicit_receive_cemented_mapping", rsnano::rsn_conf_height_unbounded_implicit_receive_cemented_mapping_size (confirmation_height_unbounded.handle), rsnano::rsn_implicit_receive_cemented_mapping_value_size () }));
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "block_cache", confirmation_height_unbounded.block_cache_size (), rsnano::rsn_conf_height_unbounded_block_cache_element_size () }));
	return composite;
}

nano::block_hash_vec::block_hash_vec () :
	handle{ rsnano::rsn_block_hash_vec_create () }
{
}

nano::block_hash_vec::block_hash_vec (rsnano::BlockHashVecHandle * handle_a) :
	handle{ handle_a }
{
}

nano::block_hash_vec::block_hash_vec (nano::block_hash_vec const & other_a) :
	handle{ rsnano::rsn_block_hash_vec_clone (other_a.handle) }
{
}

nano::block_hash_vec::~block_hash_vec ()
{
	rsnano::rsn_block_hash_vec_destroy (handle);
}
nano::block_hash_vec & nano::block_hash_vec::operator= (block_hash_vec const & other_a)
{
	rsnano::rsn_block_hash_vec_destroy (handle);
	handle = rsnano::rsn_block_hash_vec_clone (other_a.handle);
	return *this;
}
bool nano::block_hash_vec::empty () const
{
	return size () == 0;
}
size_t nano::block_hash_vec::size () const
{
	return rsnano::rsn_block_hash_vec_size (handle);
}
void nano::block_hash_vec::push_back (const nano::block_hash & hash)
{
	rsnano::rsn_block_hash_vec_push (handle, hash.bytes.data ());
}
void nano::block_hash_vec::clear ()
{
	rsnano::rsn_block_hash_vec_clear (handle);
}
void nano::block_hash_vec::assign (block_hash_vec const & source_a, size_t start, size_t end)
{
	rsnano::rsn_block_hash_vec_assign_range (handle, source_a.handle, start, end);
}
void nano::block_hash_vec::truncate (size_t new_size_a)
{
	rsnano::rsn_block_hash_vec_truncate (handle, new_size_a);
}
