#pragma once

#include <nano/lib/numbers.hpp>

namespace rsnano
{
class InactiveCacheStatusHandle;
}

namespace nano
{
class inactive_cache_status final
{
public:
	inactive_cache_status ();

	bool get_bootstrap_started () const;

	/** Did item reach config threshold to start an impromptu election? */
	bool get_election_started () const;

	/** Did item reach votes quorum? (minimum config value) */
	bool get_confirmed () const;

	nano::uint128_t get_tally () const;

	void set_bootstrap_started (bool) const;

	void set_election_started (bool) const;

	void set_confirmed (bool) const;

	void set_tally (uint8_t *) const;

	rsnano::InactiveCacheStatusHandle * handle;

	bool operator!= (inactive_cache_status const other) const;

	std::string to_string () const;
};

}
