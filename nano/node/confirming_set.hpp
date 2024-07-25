#pragma once

#include <nano/lib/numbers.hpp>

namespace rsnano
{
class ConfirmingSetHandle;
}

namespace nano
{
class block;
class ledger;
class container_info_component;
}

namespace nano
{
/**
 * Set of blocks to be durably confirmed
 */
class confirming_set final
{
public:
	confirming_set (rsnano::ConfirmingSetHandle * handle);
	~confirming_set ();
	// Adds a block to the set of blocks to be confirmed
	void add (nano::block_hash const & hash);
	// Added blocks will remain in this set until after ledger has them marked as confirmed.
	bool exists (nano::block_hash const & hash) const;
	std::size_t size () const;

	rsnano::ConfirmingSetHandle * handle;
};
}
