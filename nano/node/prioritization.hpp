#pragma once
#include <nano/lib/numbers.hpp>
#include <nano/lib/utility.hpp>

#include <cstddef>
#include <set>
#include <vector>

namespace rsnano
{
class ValueTypeHandle;
class PrioritizationHandle;
}

namespace nano
{
class block;

/** A container for holding blocks and their arrival/creation time.
 *
 *  The container consists of a number of buckets. Each bucket holds an ordered set of 'value_type' items.
 *  The buckets are accessed in a round robin fashion. The index 'current' holds the index of the bucket to access next.
 *  When a block is inserted, the bucket to go into is determined by the account balance and the priority inside that
 *  bucket is determined by its creation/arrival time.
 *
 *  The arrival/creation time is only an approximation and it could even be wildly wrong,
 *  for example, in the event of bootstrapped blocks.
 */
class prioritization final
{
	class value_type
	{
	public:
		value_type (uint64_t, std::shared_ptr<nano::block>);
		uint64_t get_time () const;
		std::shared_ptr<nano::block> get_block () const;
		bool operator< (value_type const & other_a) const;
		bool operator== (value_type const & other_a) const;
		rsnano::ValueTypeHandle * handle;
	};

	using priority = std::set<value_type>;

	void next ();
	void seek ();

public:
	prioritization (uint64_t maximum = 250000u);
	void push (uint64_t time, std::shared_ptr<nano::block> block);
	std::shared_ptr<nano::block> top () const;
	void pop ();
	std::size_t size () const;
	std::size_t bucket_count () const;
	std::size_t bucket_size (std::size_t index) const;
	bool empty () const;
	void dump () const;
	rsnano::PrioritizationHandle * handle;

	std::unique_ptr<nano::container_info_component> collect_container_info (std::string const &);
};
}
