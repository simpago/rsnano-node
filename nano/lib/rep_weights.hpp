#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/lib/utility.hpp>

#include <memory>
#include <mutex>
#include <unordered_map>

namespace rsnano
{
class RepWeightsHandle;
}
namespace nano
{
class store;
class transaction;

class rep_weights
{
public:
	rep_weights ();
	rep_weights (rsnano::RepWeightsHandle * handle_a);
	rep_weights (rep_weights const &) = delete;
	rep_weights (rep_weights &&);
	~rep_weights ();
	rep_weights & operator= (rep_weights && other_a);
	void representation_add (nano::account const & source_rep_a, nano::uint128_t const & amount_a);
	void representation_add_dual (nano::account const & source_rep_1, nano::uint128_t const & amount_1, nano::account const & source_rep_2, nano::uint128_t const & amount_2);
	nano::uint128_t representation_get (nano::account const & account_a) const;
	std::unordered_map<nano::account, nano::uint128_t> get_rep_amounts () const;

private:
	rsnano::RepWeightsHandle * handle;
	friend std::unique_ptr<container_info_component> collect_container_info (rep_weights const &, std::string const &);
};

std::unique_ptr<container_info_component> collect_container_info (rep_weights const &, std::string const &);
}
