#include "nano/lib/rsnano.hpp"

#include <nano/node/nodeconfig.hpp>
#include <nano/node/online_reps.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/store/component.hpp>
#include <nano/store/online_weight.hpp>

nano::online_reps::online_reps (rsnano::OnlineRepsHandle * handle) :
	handle{ handle }
{
}

nano::online_reps::~online_reps ()
{
	rsnano::rsn_online_reps_destroy (handle);
}

nano::uint128_t nano::online_reps::minimum_principal_weight () const
{
	nano::amount minimum;
	rsnano::rsn_online_reps_minimum_principal_weight (handle, minimum.bytes.data ());
	return minimum.number ();
}

nano::uint128_t nano::online_reps::trended () const
{
	nano::amount trended;
	rsnano::rsn_online_reps_trended (handle, trended.bytes.data ());
	return trended.number ();
}

nano::uint128_t nano::online_reps::online () const
{
	nano::amount online;
	rsnano::rsn_online_reps_online (handle, online.bytes.data ());
	return online.number ();
}

void nano::online_reps::set_online (nano::uint128_t online_a)
{
	nano::amount online_weight{ online_a };
	rsnano::rsn_online_reps_set_online (handle, online_weight.bytes.data ());
}

uint8_t nano::online_weight_quorum ()
{
	return rsnano::rsn_online_reps_online_weight_quorum ();
}

nano::uint128_t nano::online_reps::delta () const
{
	nano::amount delta;
	rsnano::rsn_online_reps_delta (handle, delta.bytes.data ());
	return delta.number ();
}

std::vector<nano::account> nano::online_reps::list ()
{
	rsnano::U256ArrayDto dto;
	rsnano::rsn_online_reps_list (handle, &dto);
	std::vector<nano::account> result;
	result.reserve (dto.count);
	for (int i = 0; i < dto.count; ++i)
	{
		nano::account account;
		std::copy (std::begin (dto.items[i]), std::end (dto.items[i]), std::begin (account.bytes));
		result.push_back (account);
	}
	rsnano::rsn_u256_array_destroy (&dto);
	return result;
}

rsnano::OnlineRepsHandle * nano::online_reps::get_handle () const
{
	return handle;
}
