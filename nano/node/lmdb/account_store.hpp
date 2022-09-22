#pragma once

#include <nano/node/lmdb/lmdb_env.hpp>
#include <nano/secure/store.hpp>

#include <lmdb/libraries/liblmdb/lmdb.h>

namespace nano
{
namespace lmdb
{
	class account_store : public nano::account_store
	{
	public:
		explicit account_store (rsnano::LmdbAccountStoreHandle * handle_a);
		account_store (account_store const &) = delete;
		account_store (account_store &&) = delete;
		~account_store () override;
		void put (nano::write_transaction const & transaction, nano::account const & account, nano::account_info const & info) override;
		bool get (nano::transaction const & transaction_a, nano::account const & account_a, nano::account_info & info_a) override;
		void del (nano::write_transaction const & transaction_a, nano::account const & account_a) override;
		bool exists (nano::transaction const & transaction_a, nano::account const & account_a) override;
		size_t count (nano::transaction const & transaction_a) override;
		nano::store_iterator<nano::account, nano::account_info> begin (nano::transaction const & transaction_a, nano::account const & account_a) const override;
		nano::store_iterator<nano::account, nano::account_info> begin (nano::transaction const & transaction_a) const override;
		nano::store_iterator<nano::account, nano::account_info> rbegin (nano::transaction const & transaction_a) const override;
		nano::store_iterator<nano::account, nano::account_info> end () const override;
		void for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::account, nano::account_info>, nano::store_iterator<nano::account, nano::account_info>)> const & action_a) const override;
		MDB_dbi get_accounts_handle () const;

	private:
		rsnano::LmdbAccountStoreHandle * handle;
	};
}
}
