#pragma once

#include <nano/secure/common.hpp>

#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/member.hpp>
#include <boost/multi_index/sequenced_index.hpp>
#include <boost/multi_index_container.hpp>

#include <chrono>

namespace nano
{
class block_arrival_info final
{
public:
	std::chrono::steady_clock::time_point arrival;
	nano::block_hash hash;
};

// This class tracks blocks that are probably live because they arrived in a UDP packet
// This gives a fairly reliable way to differentiate between blocks being inserted via bootstrap or new, live blocks.
class block_arrival final
{
public:
	block_arrival ();
	block_arrival (nano::block_arrival const &) = delete;
	block_arrival (nano::block_arrival &&) = delete;
	~block_arrival ();
	nano::block_arrival & operator= (nano::block_arrival const &) = delete;
	nano::block_arrival & operator= (nano::block_arrival &&) = delete;
	// Return `true' to indicated an error if the block has already been inserted
	bool add (nano::block_hash const &);
	bool recent (nano::block_hash const &);
	std::size_t size ();
	std::size_t size_of_element () const;

	static std::size_t constexpr arrival_size_min = 8 * 1024;
	static std::chrono::seconds constexpr arrival_time_min = std::chrono::seconds (300);

private:
	rsnano::BlockArrivalHandle * handle;
};

std::unique_ptr<container_info_component> collect_container_info (block_arrival & block_arrival, std::string const & name);
}