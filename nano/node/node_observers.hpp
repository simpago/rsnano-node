#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/transport/transport.hpp>
#include <nano/node/vote_with_weight_info.hpp>

namespace nano
{
class election_status;
}

namespace nano::transport
{
class channel;
}

namespace nano
{
class node_observers final
{
public:
	using blocks_t = nano::observer_set<nano::election_status const &, std::vector<nano::vote_with_weight_info> const &, nano::account const &, nano::uint128_t const &, bool, bool>;
	blocks_t blocks; // Notification upon election completion or cancellation
	nano::observer_set<std::shared_ptr<nano::vote>, nano::vote_source, nano::vote_code> vote;
	nano::observer_set<nano::account const &, bool> account_balance;
};
}
