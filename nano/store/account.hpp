#pragma once

#include <nano/lib/numbers.hpp>
#include <nano/store/component.hpp>
#include <nano/store/db_val_impl.hpp>
#include <nano/store/iterator.hpp>

#include <functional>

namespace nano
{
class account_info;
class block_hash;
}
namespace nano::store
{
/**
 * Manages account storage and iteration
 */
class account
{
public:
	virtual ~account () {};
	virtual void put (store::write_transaction const &, nano::account const &, nano::account_info const &) = 0;
	virtual bool get (store::transaction const &, nano::account const &, nano::account_info &) const = 0;
	virtual bool exists (store::transaction const &, nano::account const &) = 0;
	virtual nano::store::iterator<nano::account, nano::account_info> begin (store::transaction const &, nano::account const &) const = 0;
	virtual nano::store::iterator<nano::account, nano::account_info> begin (store::transaction const &) const = 0;
	virtual nano::store::iterator<nano::account, nano::account_info> end () const = 0;
};
} // namespace nano::store
