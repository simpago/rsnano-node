#pragma once

#include <nano/lib/diagnosticsconfig.hpp>
#include <nano/lib/lmdbconfig.hpp>
#include <nano/lib/logger_mt.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/node/lmdb/account_store.hpp>
#include <nano/node/lmdb/block_store.hpp>
#include <nano/node/lmdb/confirmation_height_store.hpp>
#include <nano/node/lmdb/final_vote_store.hpp>
#include <nano/node/lmdb/frontier_store.hpp>
#include <nano/node/lmdb/lmdb_env.hpp>
#include <nano/node/lmdb/lmdb_iterator.hpp>
#include <nano/node/lmdb/lmdb_txn.hpp>
#include <nano/node/lmdb/online_weight_store.hpp>
#include <nano/node/lmdb/peer_store.hpp>
#include <nano/node/lmdb/pending_store.hpp>
#include <nano/node/lmdb/pruned_store.hpp>
#include <nano/node/lmdb/unchecked_store.hpp>
#include <nano/node/lmdb/version_store.hpp>
#include <nano/secure/common.hpp>

#include <boost/optional.hpp>

namespace boost
{
namespace filesystem
{
	class path;
}
}

namespace nano
{
using mdb_val = db_val<rsnano::MdbVal>;

class transaction;

namespace lmdb
{
	/**
	 * mdb implementation of the block store
	 */
	class store : public nano::store
	{
	private:
		bool error{ false };

	public:
		rsnano::LmdbStoreHandle * handle;

	private:
		nano::lmdb::account_store account_store;
		nano::lmdb::block_store block_store;
		nano::lmdb::confirmation_height_store confirmation_height_store;
		nano::lmdb::final_vote_store final_vote_store;
		nano::lmdb::frontier_store frontier_store;
		nano::lmdb::online_weight_store online_weight_store;
		nano::lmdb::peer_store peer_store;
		nano::lmdb::pending_store pending_store;
		nano::lmdb::pruned_store pruned_store;
		nano::lmdb::unchecked_store unchecked_store;
		nano::lmdb::version_store version_store;

		friend class nano::lmdb::account_store;
		friend class nano::lmdb::block_store;
		friend class nano::lmdb::confirmation_height_store;
		friend class nano::lmdb::final_vote_store;
		friend class nano::lmdb::frontier_store;
		friend class nano::lmdb::online_weight_store;
		friend class nano::lmdb::peer_store;
		friend class nano::lmdb::pending_store;
		friend class nano::lmdb::pruned_store;
		friend class nano::lmdb::unchecked_store;
		friend class nano::lmdb::version_store;

	public:
		store (std::shared_ptr<nano::logger_mt>, boost::filesystem::path const &, nano::ledger_constants & constants, nano::txn_tracking_config const & txn_tracking_config_a = nano::txn_tracking_config{}, std::chrono::milliseconds block_processor_batch_max_time_a = std::chrono::milliseconds (5000), nano::lmdb_config const & lmdb_config_a = nano::lmdb_config{}, bool backup_before_upgrade = false);
		~store ();
		store (store const &) = delete;
		store (store &&) = delete;
		std::unique_ptr<nano::write_transaction> tx_begin_write (std::vector<nano::tables> const & tables_requiring_lock = {}, std::vector<nano::tables> const & tables_no_lock = {}) override;
		std::unique_ptr<nano::read_transaction> tx_begin_read () const override;
		void initialize (nano::write_transaction const & transaction_a, nano::ledger_cache & ledger_cache_a, nano::ledger_constants & constants) override;
		std::string vendor_get () const override;
		void serialize_mdb_tracker (boost::property_tree::ptree &, std::chrono::milliseconds, std::chrono::milliseconds) override;
		void serialize_memory_stats (boost::property_tree::ptree &) override;
		unsigned max_block_write_batch_num () const override;
		nano::block_store & block () override;
		nano::frontier_store & frontier () override;
		nano::account_store & account () override;
		nano::pending_store & pending () override;
		nano::unchecked_store & unchecked () override;
		nano::online_weight_store & online_weight () override;
		nano::pruned_store & pruned () override;
		nano::peer_store & peer () override;
		nano::confirmation_height_store & confirmation_height () override;
		nano::final_vote_store & final_vote () override;
		nano::version_store & version () override;
		bool copy_db (boost::filesystem::path const & destination_file) override;
		void rebuild_db (nano::write_transaction const & transaction_a) override;
		bool init_error () const override;
		rsnano::LmdbStoreHandle * get_handle () const override;

	private:
		friend class mdb_block_store_supported_version_upgrades_Test;
		friend class block_store_DISABLED_change_dupsort_Test;
	};
}

template <>
void * mdb_val::data () const;
template <>
std::size_t mdb_val::size () const;
template <>
mdb_val::db_val (std::size_t size_a, void * data_a);
template <>
void mdb_val::convert_buffer_to_value ();
}
