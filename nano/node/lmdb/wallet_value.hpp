#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/secure/store.hpp>

namespace nano
{
class wallet_value
{
public:
	wallet_value () = default;
	wallet_value (nano::db_val<rsnano::MdbVal> const &);
	wallet_value (nano::raw_key const &, uint64_t);
	explicit wallet_value (rsnano::WalletValueDto const & dto_a);
	rsnano::WalletValueDto to_dto () const;
	nano::raw_key key;
	uint64_t work;
};
}
