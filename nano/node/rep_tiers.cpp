#include <nano/lib/rsnano.hpp>
#include <nano/node/rep_tiers.hpp>
#include <nano/secure/common.hpp>
#include <nano/secure/ledger.hpp>

using namespace std::chrono_literals;

nano::rep_tiers::rep_tiers (rsnano::RepTiersHandle * handle) :
	handle{ handle }
{
}

nano::rep_tiers::~rep_tiers ()
{
	rsnano::rsn_rep_tiers_destroy (handle);
}

nano::rep_tier nano::rep_tiers::tier (const nano::account & representative) const
{
	return static_cast<nano::rep_tier> (rsnano::rsn_rep_tiers_tier (handle, representative.bytes.data ()));
}
