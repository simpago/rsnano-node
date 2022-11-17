#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/node/inactive_cache_status.hpp>

#include <chrono>

/*namespace rsnano
{
class InactiveCacheInformationHandle;
}*/

namespace nano
{
class inactive_cache_information final
{
public:
	inactive_cache_information () = default;
	//inactive_cache_information (inactive_cache_information &&) = delete;
	//inactive_cache_information (inactive_cache_information const &);
	//~inactive_cache_information ();
	//nano::inactive_cache_information & operator= (const nano::inactive_cache_information &);
	inactive_cache_information (std::chrono::steady_clock::time_point arrival, nano::block_hash hash, nano::account initial_rep_a, uint64_t initial_timestamp_a, nano::inactive_cache_status status) :
		arrival (arrival),
		hash (hash),
		status (status)
	{
		voters.reserve (8);
		voters.emplace_back (initial_rep_a, initial_timestamp_a);
	}

	std::chrono::steady_clock::time_point arrival;
	nano::block_hash hash;
	nano::inactive_cache_status status;
	std::vector<std::pair<nano::account, uint64_t>> voters;
	//rsnano::InactiveCacheInformationHandle * handle;

	bool needs_eval () const
	{
		return !status.get_bootstrap_started () || !status.get_election_started () || !status.get_confirmed ();
	}

	std::string to_string () const;

	/**
	 * Inserts votes stored in this entry into an election
	 * @return number of votes inserted
	 */
	std::size_t fill (std::shared_ptr<nano::election> election) const;
};

}
