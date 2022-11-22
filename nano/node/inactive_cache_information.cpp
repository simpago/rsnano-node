#include <nano/lib/rsnanoutils.hpp>
#include <nano/node/election.hpp>
#include <nano/node/inactive_cache_information.hpp>

using namespace std::chrono;

nano::inactive_cache_information::inactive_cache_information () :
	handle (rsnano::rsn_inactive_cache_information_create ())
{
}

nano::inactive_cache_information::inactive_cache_information (std::chrono::steady_clock::time_point arrival, nano::block_hash hash, nano::account initial_rep_a, uint64_t initial_timestamp_a, nano::inactive_cache_status status) :
	handle (rsnano::rsn_inactive_cache_information_create1 ((std::chrono::duration_cast<std::chrono::milliseconds> (arrival.time_since_epoch ())).count (), hash.bytes.data (), status.handle, initial_rep_a.bytes.data (), initial_timestamp_a))
{
}

nano::inactive_cache_information::inactive_cache_information (nano::inactive_cache_information const & other_a) :
	handle (rsnano::rsn_inactive_cache_information_clone (other_a.handle))
{
}

nano::inactive_cache_information::~inactive_cache_information ()
{
	if (handle != nullptr)
		rsnano::rsn_inactive_cache_information_destroy (handle);
}

nano::inactive_cache_information & nano::inactive_cache_information::operator= (const nano::inactive_cache_information & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_inactive_cache_information_destroy (handle);

	handle = rsnano::rsn_inactive_cache_information_clone (other_a.handle);
	return *this;
}

std::chrono::steady_clock::time_point nano::inactive_cache_information::get_arrival () const
{
	auto value = rsnano::rsn_inactive_cache_information_get_arrival (handle);
	return std::chrono::steady_clock::time_point (std::chrono::steady_clock::duration (value));
}

nano::block_hash nano::inactive_cache_information::get_hash () const
{
	nano::block_hash result;
	rsnano::rsn_inactive_cache_information_get_hash (handle, result.bytes.data ());
	return result;
}

nano::inactive_cache_status nano::inactive_cache_information::get_status () const
{
	rsnano::InactiveCacheStatusHandle * status_handle = rsnano::rsn_inactive_cache_information_get_status (handle);
	if (status_handle == nullptr)
		return nano::inactive_cache_status ();

	nano::inactive_cache_status status = nano::inactive_cache_status ();
	status.set_bootstrap_started (rsnano::rsn_inactive_cache_status_bootstrap_started (status_handle));
	status.set_election_started (rsnano::rsn_inactive_cache_status_election_started (status_handle));
	status.set_confirmed (rsnano::rsn_inactive_cache_status_confirmed (status_handle));
	uint8_t * result;
	rsnano::rsn_inactive_cache_status_tally (status_handle, result);
	status.set_tally (result);
	return status;
}

std::vector<std::pair<nano::account, uint64_t>> nano::inactive_cache_information::get_voters () const
{
	rsnano::VotersDto voters_dto;
	rsnano::rsn_inactive_cache_information_get_voters (handle, &voters_dto);
	std::vector<std::pair<nano::account, uint64_t>> voters;
	rsnano::VotersItemDto const * current;
	int i;
	for (i = 0, current = voters_dto.items; i < voters_dto.count; ++i)
	{
		uint64_t timestamp = current->timestamp;
		const uint8_t * account = current->account;
		uint8_t * a = const_cast<uint8_t *> (account);
		nano::uint256_t result;
		boost::multiprecision::export_bits (result, a, 8, false);
		uint256_union b = uint256_union (result);
		public_key c = b;
		voters.push_back (std::make_pair (nano::account (c.to_account ()), timestamp));
		current++;
	}

	rsnano::rsn_inactive_cache_information_destroy_dto (&voters_dto);

	return voters;
}

std::string nano::inactive_cache_information::to_string () const
{
	rsnano::StringDto result;
	rsnano::rsn_inactive_cache_information_to_string (handle, &result);
	return rsnano::convert_dto_to_string (result);
}

std::size_t nano::inactive_cache_information::fill (std::shared_ptr<nano::election> election) const
{
	std::size_t inserted = 0;
	for (auto const & [rep, timestamp] : get_voters ())
	{
		auto [is_replay, processed] = election->vote (rep, timestamp, get_hash (), nano::election::vote_source::cache);
		if (processed)
		{
			inserted++;
		}
	}
	return inserted;
}