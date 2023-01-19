#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/lib/utility.hpp>
#include <nano/secure/common.hpp>

#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/member.hpp>
#include <boost/multi_index/ordered_index.hpp>
#include <boost/multi_index_container.hpp>

#include <memory>
#include <vector>

namespace nano
{
class ledger;
class node_config;
class transaction;

/** Track online representatives and trend online weight */
class online_reps final
{
public:
	online_reps (nano::ledger & ledger_a, nano::node_config const & config_a);
	~online_reps ();
	/** Add voting account \p rep_account to the set of online representatives */
	void observe (nano::account const & rep_account);
	/** Called periodically to sample online weight */
	void sample ();
	/** Returns the trended online stake */
	nano::uint128_t trended () const;
	/** Returns the current online stake */
	nano::uint128_t online () const;
	/** Returns the quorum required for confirmation*/
	nano::uint128_t delta () const;
	/** List of online representatives, both the currently sampling ones and the ones observed in the previous sampling period */
	std::vector<nano::account> list ();
	void clear ();
	static uint8_t online_weight_quorum ();
	void set_online (nano::uint128_t);

private:
	rsnano::OnlineRepsHandle * handle;

	friend class election_quorum_minimum_update_weight_before_quorum_checks_Test;
	friend std::unique_ptr<container_info_component> collect_container_info (online_reps & online_reps, std::string const & name);
};

std::unique_ptr<container_info_component> collect_container_info (online_reps & online_reps, std::string const & name);
}
