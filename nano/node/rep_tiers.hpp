#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/lib/utility.hpp>
#include <nano/secure/common.hpp>

namespace rsnano
{
class RepTiersHandle;
}

namespace nano
{
class ledger;
class network_params;
class stats;
class logger;
class container_info_component;

// Higher number means higher priority
enum class rep_tier
{
	none, // Not a principal representatives
	tier_1, // (0.1-1%) of online stake
	tier_2, // (1-5%) of online stake
	tier_3, // (> 5%) of online stake
};

class rep_tiers final
{
public:
	rep_tiers (rsnano::RepTiersHandle * handle);
	~rep_tiers ();

	/** Returns the representative tier for the account */
	nano::rep_tier tier (nano::account const & representative) const;

	rsnano::RepTiersHandle * handle;
};
}
