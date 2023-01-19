#pragma once

#include <nano/lib/diagnosticsconfig.hpp>
#include <nano/lib/timer.hpp>
#include <nano/secure/store.hpp>

#include <boost/property_tree/ptree_fwd.hpp>
#include <boost/stacktrace/stacktrace_fwd.hpp>

namespace nano
{
class transaction;
class logger_mt;
class mdb_env;

class mdb_txn_callbacks
{
public:
	// takes a txn_id and is_write
	std::function<void (uint64_t, bool)> txn_start{ [] (uint64_t, bool) {} };

	// takes a txn_id
	std::function<void (uint64_t)> txn_end{ [] (uint64_t) {} };
};

class read_mdb_txn final : public read_transaction
{
public:
	read_mdb_txn (read_mdb_txn const &) = delete;
	read_mdb_txn (read_mdb_txn &&) = delete;
	read_mdb_txn (rsnano::TransactionHandle * handle_a);
	~read_mdb_txn () override;
	void reset () override;
	void renew () override;
	void refresh () override;
	rsnano::TransactionHandle * get_rust_handle () const override
	{
		return txn_handle;
	};

	rsnano::TransactionHandle * txn_handle;
};

class write_mdb_txn final : public write_transaction
{
public:
	write_mdb_txn (write_mdb_txn const &) = delete;
	write_mdb_txn (write_mdb_txn &&) = delete;
	write_mdb_txn (rsnano::TransactionHandle * handle_a);
	~write_mdb_txn () override;
	void commit () override;
	void renew () override;
	void refresh () override;
	bool contains (nano::tables table_a) const override;
	rsnano::TransactionHandle * get_rust_handle () const override
	{
		return txn_handle;
	};

	rsnano::TransactionHandle * txn_handle;
};
}
