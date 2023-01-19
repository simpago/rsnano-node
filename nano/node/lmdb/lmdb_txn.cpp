#include <nano/lib/jsonconfig.hpp>
#include <nano/lib/logger_mt.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/lmdb/lmdb_env.hpp>
#include <nano/node/lmdb/lmdb_txn.hpp>
#include <nano/secure/store.hpp>

#include <boost/format.hpp>

#ifdef _WIN32
#ifndef NOMINMAX
#define NOMINMAX
#endif
#endif
#include <boost/stacktrace.hpp>

nano::read_mdb_txn::read_mdb_txn (rsnano::TransactionHandle * handle_a) :
	txn_handle{ handle_a }
{
}

nano::read_mdb_txn::~read_mdb_txn ()
{
	rsnano::rsn_lmdb_read_txn_destroy (txn_handle);
}

void nano::read_mdb_txn::reset ()
{
	rsnano::rsn_lmdb_read_txn_reset (txn_handle);
}

void nano::read_mdb_txn::renew ()
{
	rsnano::rsn_lmdb_read_txn_renew (txn_handle);
}

void nano::read_mdb_txn::refresh ()
{
	rsnano::rsn_lmdb_read_txn_refresh (txn_handle);
}

nano::write_mdb_txn::write_mdb_txn (rsnano::TransactionHandle * handle_a) :
	txn_handle{ handle_a }
{
}

nano::write_mdb_txn::~write_mdb_txn ()
{
	rsnano::rsn_lmdb_write_txn_destroy (txn_handle);
}

void nano::write_mdb_txn::commit ()
{
	rsnano::rsn_lmdb_write_txn_commit (txn_handle);
}

void nano::write_mdb_txn::renew ()
{
	rsnano::rsn_lmdb_write_txn_renew (txn_handle);
}

void nano::write_mdb_txn::refresh ()
{
	rsnano::rsn_lmdb_write_txn_refresh (txn_handle);
}

bool nano::write_mdb_txn::contains (nano::tables table_a) const
{
	// LMDB locks on every write
	return true;
}
