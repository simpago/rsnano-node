#include "nano/lib/rsnano.hpp"

#include <nano/node/inactive_cache_status.hpp>

nano::inactive_cache_status::inactive_cache_status () :
	handle (rsnano::rsn_inactive_cache_status_create ())
{
}

bool nano::inactive_cache_status::get_bootstrap_started () const
{
	return rsnano::rsn_inactive_cache_status_bootstrap_started (handle);
}

bool nano::inactive_cache_status::get_election_started () const
{
	return rsnano::rsn_inactive_cache_status_election_started (handle);
}

bool nano::inactive_cache_status::get_confirmed () const
{
	return rsnano::rsn_inactive_cache_status_confirmed (handle);
}

nano::uint128_t nano::inactive_cache_status::get_tally () const
{
	nano::uint128_t tally;
	uint8_t * rsn_tally;
	rsnano::rsn_inactive_cache_status_tally (handle, rsn_tally);
	boost::multiprecision::export_bits (tally, rsn_tally, 8, false);
	return tally;
}

void nano::inactive_cache_status::set_bootstrap_started (bool bootstrap_started) const
{
	rsnano::rsn_inactive_cache_status_set_bootstrap_started (handle, bootstrap_started);
}

void nano::inactive_cache_status::set_election_started (bool election_started) const
{
	rsnano::rsn_inactive_cache_status_set_election_started (handle, election_started);
}

void nano::inactive_cache_status::set_confirmed (bool confirmed) const
{
	rsnano::rsn_inactive_cache_status_set_confirmed (handle, confirmed);
}

void nano::inactive_cache_status::set_tally (uint8_t * tally) const
{
	rsnano::rsn_inactive_cache_status_set_tally (handle, tally);
}

bool nano::inactive_cache_status::operator!= (inactive_cache_status const other) const
{
	uint8_t * rsn_tally;
	nano::uint128_t other_tally;
	rsnano::rsn_inactive_cache_status_tally (other.handle, rsn_tally);
	boost::multiprecision::export_bits (other_tally, rsn_tally, 8, false);

	return rsnano::rsn_inactive_cache_status_bootstrap_started (handle) != rsnano::rsn_inactive_cache_status_bootstrap_started (other.handle)
	|| rsnano::rsn_inactive_cache_status_election_started (handle) != rsnano::rsn_inactive_cache_status_election_started (other.handle)
	|| rsnano::rsn_inactive_cache_status_confirmed (handle) != rsnano::rsn_inactive_cache_status_confirmed (other.handle)
	|| get_tally () != other_tally;
}

std::string nano::inactive_cache_status::to_string () const
{
	rsnano::rsn_inactive_cache_status_to_string (handle);
}
