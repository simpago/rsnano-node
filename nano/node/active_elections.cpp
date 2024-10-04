#include "nano/lib/rsnano.hpp"
#include "nano/lib/utility.hpp"
#include "nano/node/election_status.hpp"

#include <nano/lib/blocks.hpp>
#include <nano/lib/threading.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/confirming_set.hpp>
#include <nano/node/election.hpp>
#include <nano/node/node.hpp>
#include <nano/node/repcrawler.hpp>
#include <nano/node/scheduler/component.hpp>
#include <nano/node/scheduler/priority.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/store/component.hpp>

#include <boost/format.hpp>

#include <cstdint>
#include <memory>

using namespace std::chrono;

namespace
{
void call_vacancy_update (void * context)
{
	auto callback = static_cast<std::function<void ()> *> (context);
	(*callback) ();
}

void delete_vacancy_update (void * context)
{
	auto callback = static_cast<std::function<void ()> *> (context);
	delete callback;
}

void call_vote_processed (void * context, rsnano::VoteHandle * vote_handle, uint8_t source, rsnano::VoteResultMapHandle * results_handle)
{
	auto callback = static_cast<std::function<void (std::shared_ptr<nano::vote> const &, nano::vote_source, std::unordered_map<nano::block_hash, nano::vote_code> const &)> *> (context);
	auto vote = std::make_shared<nano::vote> (vote_handle);
	std::unordered_map<nano::block_hash, nano::vote_code> result;
	auto len = rsnano::rsn_vote_result_map_len (results_handle);
	for (auto i = 0; i < len; ++i)
	{
		nano::block_hash hash;
		auto code = rsnano::rsn_vote_result_map_get (results_handle, i, hash.bytes.data ());
		result.emplace (hash, static_cast<nano::vote_code> (code));
	}
	rsnano::rsn_vote_result_map_destroy (results_handle);
	(*callback) (vote, static_cast<nano::vote_source> (source), result);
}

void delete_vote_processed_context (void * context)
{
	auto callback = static_cast<std::function<void (std::shared_ptr<nano::vote> const &, nano::vote_source, std::unordered_map<nano::block_hash, nano::vote_code> const &)> *> (context);
	delete callback;
}

}

nano::active_elections::active_elections (nano::node & node_a, rsnano::ActiveTransactionsHandle * handle) :
	handle{ handle },
	node{ node_a }
{
}

nano::active_elections::~active_elections ()
{
	rsnano::rsn_active_transactions_destroy (handle);
}

void nano::active_elections::stop ()
{
	rsnano::rsn_active_transactions_stop (handle);
}

bool nano::active_elections::confirmed (nano::election const & election) const
{
	return rsnano::rsn_active_transactions_confirmed (handle, election.handle);
}

std::vector<nano::vote_with_weight_info> nano::active_elections::votes_with_weight (nano::election & election) const
{
	std::multimap<nano::uint128_t, nano::vote_with_weight_info, std::greater<nano::uint128_t>> sorted_votes;
	std::vector<nano::vote_with_weight_info> result;
	auto votes_l (election.votes ());
	for (auto const & vote_l : votes_l)
	{
		if (vote_l.first != nullptr)
		{
			auto amount = node.get_rep_weight (vote_l.first);
			nano::vote_with_weight_info vote_info{ vote_l.first, vote_l.second.get_time (), vote_l.second.get_timestamp (), vote_l.second.get_hash (), amount.number () };
			sorted_votes.emplace (amount.number (), vote_info);
		}
		else
		{
		}
	}
	result.reserve (sorted_votes.size ());
	std::transform (sorted_votes.begin (), sorted_votes.end (), std::back_inserter (result), [] (auto const & entry) { return entry.second; });
	return result;
}

void nano::active_elections::add_election_winner_details (nano::block_hash const & hash_a, std::shared_ptr<nano::election> const & election_a)
{
	rsnano::rsn_active_transactions_add_election_winner_details (handle, hash_a.bytes.data (), election_a->handle);
}

void nano::active_elections::process_confirmed (nano::election_status const & status_a, uint64_t iteration_a)
{
	rsnano::rsn_active_transactions_process_confirmed (handle, status_a.handle, iteration_a);
}

nano::tally_t nano::active_elections::tally_impl (nano::election_lock & lock) const
{
	nano::tally_t result;
	auto tally_handle = rsnano::rsn_active_transactions_tally_impl (handle, lock.handle);
	for (size_t i = 0, n = rsnano::rsn_tally_blocks_len (tally_handle); i < n; ++i)
	{
		nano::amount weight;
		auto block_handle = rsnano::rsn_tally_blocks_get (tally_handle, i, weight.bytes.data ());
		result.emplace (weight.number (), nano::block_handle_to_block (block_handle));
	}
	rsnano::rsn_tally_blocks_destroy (tally_handle);
	return result;
}

void nano::active_elections::force_confirm (nano::election & election)
{
	rsnano::rsn_active_transactions_force_confirm (handle, election.handle);
}

int64_t nano::active_elections::limit (nano::election_behavior behavior) const
{
	return rsnano::rsn_active_transactions_limit (handle, static_cast<uint8_t> (behavior));
}

int64_t nano::active_elections::vacancy (nano::election_behavior behavior) const
{
	return rsnano::rsn_active_transactions_vacancy (handle, static_cast<uint8_t> (behavior));
}

std::vector<std::shared_ptr<nano::election>> nano::active_elections::list_active (std::size_t max_a)
{
	std::vector<std::shared_ptr<nano::election>> result_l;
	auto elections_handle = rsnano::rsn_active_transactions_list_active (handle, max_a);
	auto len = rsnano::rsn_election_vec_len (elections_handle);
	result_l.reserve (std::min (max_a, len));
	std::size_t count_l{ 0 };
	for (auto i = 0; i < len && count_l < max_a; ++i, ++count_l)
	{
		auto election = std::make_shared<nano::election> (rsnano::rsn_election_vec_get (elections_handle, i));
		result_l.push_back (election);
	}
	rsnano::rsn_election_vec_destroy (elections_handle);
	return result_l;
}

nano::election_extended_status nano::active_elections::current_status (nano::election & election) const
{
	nano::election_lock guard{ election };
	nano::election_status status_l = guard.status ();
	status_l.set_confirmation_request_count (election.get_confirmation_request_count ());
	status_l.set_block_count (nano::narrow_cast<decltype (status_l.get_block_count ())> (guard.last_blocks_size ()));
	status_l.set_voter_count (nano::narrow_cast<decltype (status_l.get_voter_count ())> (guard.last_votes_size ()));
	return nano::election_extended_status{ status_l, guard.last_votes (), tally_impl (guard) };
}

nano::tally_t nano::active_elections::tally (nano::election & election) const
{
	auto guard{ election.lock () };
	return tally_impl (guard);
}

void nano::active_elections::clear_recently_confirmed ()
{
	rsnano::rsn_active_transactions_clear_recently_confirmed (handle);
}

std::size_t nano::active_elections::recently_confirmed_size ()
{
	return rsnano::rsn_active_transactions_recently_confirmed_count (handle);
}

std::size_t nano::active_elections::recently_cemented_size ()
{
	return rsnano::rsn_active_transactions_recently_cemented_count (handle);
}

nano::qualified_root nano::active_elections::lastest_recently_confirmed_root ()
{
	nano::qualified_root result;
	rsnano::rsn_active_transactions_latest_recently_confirmed_root (handle, result.bytes.data ());
	return result;
}

void nano::active_elections::insert_recently_confirmed (std::shared_ptr<nano::block> const & block)
{
	rsnano::rsn_active_transactions_recently_confirmed_insert (handle, block->get_handle ());
}

void nano::active_elections::insert_recently_cemented (nano::election_status const & status)
{
	rsnano::rsn_active_transactions_recently_cemented_insert (handle, status.handle);
}

std::deque<nano::election_status> nano::active_elections::recently_cemented_list ()
{
	rsnano::RecentlyCementedCachedDto recently_cemented_cache_dto;
	rsnano::rsn_active_transactions_recently_cemented_list (handle, &recently_cemented_cache_dto);
	std::deque<nano::election_status> result;
	rsnano::ElectionStatusHandle * const * current;
	int i;
	for (i = 0, current = recently_cemented_cache_dto.items; i < recently_cemented_cache_dto.count; ++i)
	{
		nano::election_status election_status (*current);
		result.push_back (election_status);
		current++;
	}

	rsnano::rsn_recently_cemented_cache_destroy_dto (&recently_cemented_cache_dto);

	return result;
}

bool nano::active_elections::active (nano::qualified_root const & root_a) const
{
	return rsnano::rsn_active_transactions_active_root (handle, root_a.bytes.data ());
}

bool nano::active_elections::active (nano::block const & block_a) const
{
	return rsnano::rsn_active_transactions_active (handle, block_a.get_handle ());
}

std::shared_ptr<nano::election> nano::active_elections::election (nano::qualified_root const & root_a) const
{
	std::shared_ptr<nano::election> result;
	auto election_handle = rsnano::rsn_active_transactions_election (handle, root_a.bytes.data ());
	if (election_handle != nullptr)
	{
		result = std::make_shared<nano::election> (election_handle);
	}
	return result;
}

bool nano::active_elections::erase (nano::block const & block_a)
{
	return erase (block_a.qualified_root ());
}

bool nano::active_elections::erase (nano::qualified_root const & root_a)
{
	return rsnano::rsn_active_transactions_erase (handle, root_a.bytes.data ());
}

bool nano::active_elections::empty () const
{
	return size () == 0;
}

std::size_t nano::active_elections::size () const
{
	return rsnano::rsn_active_transactions_len (handle);
}

bool nano::active_elections::publish (std::shared_ptr<nano::block> const & block_a)
{
	return rsnano::rsn_active_transactions_publish_block (handle, block_a->get_handle ());
}

nano::vote_code nano::active_elections::vote (nano::election & election, nano::account const & rep, uint64_t timestamp_a, nano::block_hash const & block_hash_a, nano::vote_source vote_source_a)
{
	auto result = rsnano::rsn_active_transactions_vote2 (handle, election.handle, rep.bytes.data (), timestamp_a, block_hash_a.bytes.data (), static_cast<uint8_t> (vote_source_a));
	return static_cast<nano::vote_code> (result);
}

std::size_t nano::active_elections::election_winner_details_size ()
{
	return rsnano::rsn_active_transactions_election_winner_details_len (handle);
}

void nano::active_elections::clear ()
{
	rsnano::rsn_active_transactions_clear (handle);
}

/*
 * active_transactions_config
 */

nano::active_elections_config::active_elections_config (rsnano::ActiveElectionsConfigDto const & dto) :
	size{ dto.size },
	hinted_limit_percentage{ dto.hinted_limit_percentage },
	optimistic_limit_percentage{ dto.optimistic_limit_percentage },
	confirmation_history_size{ dto.confirmation_history_size },
	confirmation_cache{ dto.confirmation_cache },
	max_election_winners{ dto.max_election_winners }
{
}

rsnano::ActiveElectionsConfigDto nano::active_elections_config::into_dto () const
{
	return { size, hinted_limit_percentage, optimistic_limit_percentage, confirmation_history_size, confirmation_cache, max_election_winners };
}

nano::error nano::active_elections_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("size", size);
	toml.get ("hinted_limit_percentage", hinted_limit_percentage);
	toml.get ("optimistic_limit_percentage", optimistic_limit_percentage);
	toml.get ("confirmation_history_size", confirmation_history_size);
	toml.get ("confirmation_cache", confirmation_cache);

	return toml.get_error ();
}
