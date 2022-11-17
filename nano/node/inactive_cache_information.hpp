#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/node/inactive_cache_status.hpp>

#include <chrono>

namespace nano
{
class inactive_cache_information final
{
public:
	inactive_cache_information (); // = default;
	inactive_cache_information (inactive_cache_information &&) = delete;
	inactive_cache_information (inactive_cache_information const &);
	~inactive_cache_information ();
	nano::inactive_cache_information & operator= (const nano::inactive_cache_information &);
	inactive_cache_information (std::chrono::steady_clock::time_point arrival, nano::block_hash hash, nano::account initial_rep_a, uint64_t initial_timestamp_a, nano::inactive_cache_status status);

	std::chrono::steady_clock::time_point get_arrival () const;
	nano::block_hash get_hash () const;
	nano::inactive_cache_status get_status () const;
	std::vector<std::pair<nano::account, uint64_t>> get_voters () const;
	rsnano::InactiveCacheInformationHandle * handle;

	bool needs_eval () const
	{
		return !get_status ().get_bootstrap_started () || !get_status ().get_election_started () || !get_status ().get_confirmed ();
	}

	std::string to_string () const;

	/**
	 * Inserts votes stored in this entry into an election
	 * @return number of votes inserted
	 */
	std::size_t fill (std::shared_ptr<nano::election> election) const;
};

}
