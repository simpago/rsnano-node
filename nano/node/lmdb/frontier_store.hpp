#pragma once

#include <nano/secure/store.hpp>

#include <lmdb/libraries/liblmdb/lmdb.h>

namespace nano
{
namespace lmdb
{
	class store;
	class frontier_store : public nano::frontier_store
	{
	private:
		nano::lmdb::store & store;
		rsnano::LmdbFrontierStoreHandle * handle;

	public:
		frontier_store (nano::lmdb::store & store);
		~frontier_store ();
		frontier_store (frontier_store const &) = delete;
		frontier_store (frontier_store &&) = delete;
		void put (nano::write_transaction const &, nano::block_hash const &, nano::account const &) override;
		nano::account get (nano::transaction const &, nano::block_hash const &) const override;
		void del (nano::write_transaction const &, nano::block_hash const &) override;
		nano::store_iterator<nano::block_hash, nano::account> begin (nano::transaction const &) const override;
		nano::store_iterator<nano::block_hash, nano::account> begin (nano::transaction const &, nano::block_hash const &) const override;
		nano::store_iterator<nano::block_hash, nano::account> end () const override;
		void for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::block_hash, nano::account>, nano::store_iterator<nano::block_hash, nano::account>)> const & action_a) const override;

		MDB_dbi table_handle () const;
		void set_table_handle (MDB_dbi dbi);
	};
}
}
