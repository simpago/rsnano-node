#include <nano/node/nodeconfig.hpp>
#include <nano/node/online_reps.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/secure/store.hpp>

#include <iostream>

nano::online_reps::online_reps (nano::ledger & ledger_a, nano::node_config const & config_a) :
	handle{ rsnano::rsn_online_reps_create (ledger_a.get_handle (), config_a.to_dto ()) }
{
}

nano::online_reps::~online_reps ()
{
	if (handle)
		rsnano::rsn_online_reps_destroy (handle);
}

void nano::online_reps::observe (nano::account const & rep_a)
{
	rsnano::rsn_online_reps_observe (handle, rep_a.bytes.data ());
}

void nano::online_reps::sample ()
{
	rsnano::rsn_online_reps_sample (handle);
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

void nano::online_reps::set_online (nano::uint128_t online)
{
	uint8_t bytes[16] = { 0 };
	boost::multiprecision::export_bits (online, std::rbegin (bytes), 8, false);
	rsnano::rsn_online_reps_set_online (handle, &bytes[0]);
}

uint8_t nano::online_reps::online_weight_quorum ()
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

void nano::online_reps::clear ()
{
	rsnano::rsn_online_reps_clear (handle);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (online_reps & online_reps, std::string const & name)
{
	size_t count = rsnano::rsn_online_reps_item_count (online_reps.handle);
	auto sizeof_element = rsnano::rsn_online_reps_item_size ();
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "reps", count, sizeof_element }));
	return composite;
}
