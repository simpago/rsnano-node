#include <nano/node/lmdb/lmdb.hpp>
#include <nano/node/lmdb/peer_store.hpp>

namespace
{
nano::store_iterator<nano::endpoint_key, nano::no_value> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return { nullptr };
	}

	return { std::make_unique<nano::mdb_iterator<nano::endpoint_key, nano::no_value>> (it_handle) };
}
}

nano::lmdb::peer_store::peer_store (rsnano::LmdbPeerStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::peer_store::~peer_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_peer_store_destroy (handle);
}

void nano::lmdb::peer_store::put (nano::write_transaction const & transaction, nano::endpoint_key const & endpoint)
{
	rsnano::rsn_lmdb_peer_store_put (handle, transaction.get_rust_handle (), endpoint.address_bytes ().data (), endpoint.port ());
}

void nano::lmdb::peer_store::del (nano::write_transaction const & transaction, nano::endpoint_key const & endpoint)
{
	rsnano::rsn_lmdb_peer_store_del (handle, transaction.get_rust_handle (), endpoint.address_bytes ().data (), endpoint.port ());
}

bool nano::lmdb::peer_store::exists (nano::transaction const & transaction, nano::endpoint_key const & endpoint) const
{
	return rsnano::rsn_lmdb_peer_store_exists (handle, transaction.get_rust_handle (), endpoint.address_bytes ().data (), endpoint.port ());
}

size_t nano::lmdb::peer_store::count (nano::transaction const & transaction) const
{
	return rsnano::rsn_lmdb_peer_store_count (handle, transaction.get_rust_handle ());
}

void nano::lmdb::peer_store::clear (nano::write_transaction const & transaction)
{
	rsnano::rsn_lmdb_peer_store_clear (handle, transaction.get_rust_handle ());
}

nano::store_iterator<nano::endpoint_key, nano::no_value> nano::lmdb::peer_store::begin (nano::transaction const & transaction) const
{
	auto it_handle{ rsnano::rsn_lmdb_peer_store_begin (handle, transaction.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::endpoint_key, nano::no_value> nano::lmdb::peer_store::end () const
{
	return nano::store_iterator<nano::endpoint_key, nano::no_value> (nullptr);
}
