#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/timer.hpp>
#include <nano/secure/store.hpp>

#include <chrono>
#include <unordered_map>

namespace nano
{
class ledger;
class read_transaction;
class logging;
class logger_mt;
class write_database_queue;
class write_guard;

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

class confirmation_height_unbounded final
{
public:
	confirmation_height_unbounded (nano::ledger &, nano::stat &, nano::write_database_queue &, std::chrono::milliseconds batch_separate_pending_min_time, nano::logging const &, std::shared_ptr<nano::logger_mt> &, rsnano::AtomicU64Wrapper & batch_write_size, std::function<void (std::vector<std::shared_ptr<nano::block>> const &)> const & cemented_callback, std::function<void (nano::block_hash const &)> const & already_cemented_callback, std::function<uint64_t ()> const & awaiting_processing_size_query);
	confirmation_height_unbounded (confirmation_height_unbounded const &) = delete;
	confirmation_height_unbounded (confirmation_height_unbounded &&) = delete;
	~confirmation_height_unbounded ();
	bool pending_empty () const;
	void clear_process_vars ();
	void process (std::shared_ptr<nano::block> original_block);
	void cement_blocks ();
	bool has_iterated_over_block (nano::block_hash const &) const;
	uint64_t block_cache_size () const;
	void stop ();
	size_t pending_writes_size () const;

	rsnano::ConfirmationHeightUnboundedHandle * handle;

private:
	friend class confirmation_height_dynamic_algorithm_no_transition_while_pending_Test;
	friend std::unique_ptr<nano::container_info_component> collect_container_info (confirmation_height_unbounded &, std::string const & name_a);
};

std::unique_ptr<nano::container_info_component> collect_container_info (confirmation_height_unbounded &, std::string const & name_a);
}
