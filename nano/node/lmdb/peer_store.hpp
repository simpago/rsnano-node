#pragma once

#include <nano/secure/store.hpp>

#include <lmdb/libraries/liblmdb/lmdb.h>

namespace nano
{
namespace lmdb
{
	class store;
	class peer_store : public nano::peer_store
	{
	private:
		rsnano::LmdbPeerStoreHandle * handle;

	public:
		explicit peer_store (rsnano::LmdbPeerStoreHandle * handle_a);
		~peer_store ();
		peer_store (peer_store const &) = delete;
		peer_store (peer_store &&) = delete;
		void put (nano::write_transaction const & transaction_a, nano::endpoint_key const & endpoint_a) override;
		void del (nano::write_transaction const & transaction_a, nano::endpoint_key const & endpoint_a) override;
		bool exists (nano::transaction const & transaction_a, nano::endpoint_key const & endpoint_a) const override;
		size_t count (nano::transaction const & transaction_a) const override;
		void clear (nano::write_transaction const & transaction_a) override;
		nano::store_iterator<nano::endpoint_key, nano::no_value> begin (nano::transaction const & transaction_a) const override;
		nano::store_iterator<nano::endpoint_key, nano::no_value> end () const override;

		MDB_dbi table_handle () const;
	};
}
}
