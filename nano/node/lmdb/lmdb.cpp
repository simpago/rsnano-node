#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/lmdb/lmdb.hpp>
#include <nano/node/lmdb/lmdb_iterator.hpp>

#include <boost/filesystem.hpp>
#include <boost/format.hpp>
#include <boost/polymorphic_cast.hpp>

#include <queue>

namespace nano
{
template <>
void * mdb_val::data () const
{
	return value.mv_data;
}

template <>
std::size_t mdb_val::size () const
{
	return value.mv_size;
}

template <>
mdb_val::db_val (std::size_t size_a, void * data_a) :
	value ({ size_a, data_a })
{
}

template <>
void mdb_val::convert_buffer_to_value ()
{
	value = { buffer->size (), const_cast<uint8_t *> (buffer->data ()) };
}
}
namespace
{
rsnano::LmdbStoreHandle * create_store_handle (bool & error_a, boost::filesystem::path const & path_a, nano::mdb_env::options options_a, const std::shared_ptr<nano::logger_mt> & logger_a, nano::txn_tracking_config const & txn_tracking_config_a, std::chrono::milliseconds block_processor_batch_max_time_a, bool backup_before_upgrade)
{
	auto path_string{ path_a.string () };
	auto config_dto{ options_a.config.to_dto () };
	auto txn_config_dto{ txn_tracking_config_a.to_dto () };
	return rsnano::rsn_lmdb_store_create (&error_a, reinterpret_cast<const int8_t *> (path_string.c_str ()), &config_dto, options_a.use_no_mem_init, nano::to_logger_handle (logger_a), &txn_config_dto, block_processor_batch_max_time_a.count (), backup_before_upgrade);
}
}

nano::lmdb::store::store (std::shared_ptr<nano::logger_mt> logger_a, boost::filesystem::path const & path_a, nano::ledger_constants & constants, nano::txn_tracking_config const & txn_tracking_config_a, std::chrono::milliseconds block_processor_batch_max_time_a, nano::lmdb_config const & lmdb_config_a, bool backup_before_upgrade_a) :
	handle{ create_store_handle (error, path_a, nano::mdb_env::options::make ().set_config (lmdb_config_a).set_use_no_mem_init (true), logger_a, txn_tracking_config_a, block_processor_batch_max_time_a, backup_before_upgrade_a) },
	block_store{ rsnano::rsn_lmdb_store_block (handle) },
	frontier_store{ rsnano::rsn_lmdb_store_frontier (handle) },
	account_store{ rsnano::rsn_lmdb_store_account (handle) },
	pending_store{ rsnano::rsn_lmdb_store_pending (handle) },
	online_weight_store{ rsnano::rsn_lmdb_store_online_weight (handle) },
	pruned_store{ rsnano::rsn_lmdb_store_pruned (handle) },
	peer_store{ rsnano::rsn_lmdb_store_peer (handle) },
	confirmation_height_store{ rsnano::rsn_lmdb_store_confirmation_height (handle) },
	final_vote_store{ rsnano::rsn_lmdb_store_final_vote (handle) },
	unchecked_store{ rsnano::rsn_lmdb_store_unchecked (handle) },
	version_store{ rsnano::rsn_lmdb_store_version (handle) }
{
}

nano::lmdb::store::~store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_store_destroy (handle);
}

void nano::lmdb::store::serialize_mdb_tracker (boost::property_tree::ptree & json, std::chrono::milliseconds min_read_time, std::chrono::milliseconds min_write_time)
{
	rsnano::rsn_lmdb_store_serialize_mdb_tracker (handle, &json, min_read_time.count (), min_write_time.count ());
}

void nano::lmdb::store::serialize_memory_stats (boost::property_tree::ptree & json)
{
	rsnano::rsn_lmdb_store_serialize_memory_stats (handle, &json);
}

std::unique_ptr<nano::write_transaction> nano::lmdb::store::tx_begin_write (std::vector<nano::tables> const &, std::vector<nano::tables> const &)
{
	return std::make_unique<nano::write_mdb_txn> (rsnano::rsn_lmdb_store_tx_begin_write (handle));
}

std::unique_ptr<nano::read_transaction> nano::lmdb::store::tx_begin_read () const
{
	return std::make_unique<nano::read_mdb_txn> (rsnano::rsn_lmdb_store_tx_begin_read (handle));
}

std::string nano::lmdb::store::vendor_get () const
{
	rsnano::StringDto dto;
	rsnano::rsn_lmdb_store_vendor_get (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

bool nano::lmdb::store::copy_db (boost::filesystem::path const & destination_file)
{
	return !rsnano::rsn_lmdb_store_copy_db (handle, reinterpret_cast<const int8_t *> (destination_file.string ().c_str ()));
}

void nano::lmdb::store::rebuild_db (nano::write_transaction const & transaction_a)
{
	rsnano::rsn_lmdb_store_rebuild_db (handle, transaction_a.get_rust_handle ());
}

bool nano::lmdb::store::init_error () const
{
	return error;
}

unsigned nano::lmdb::store::max_block_write_batch_num () const
{
	return std::numeric_limits<unsigned>::max ();
}

nano::block_store & nano::lmdb::store::block ()
{
	return block_store;
}

nano::frontier_store & nano::lmdb::store::frontier ()
{
	return frontier_store;
}

nano::account_store & nano::lmdb::store::account ()
{
	return account_store;
}

nano::pending_store & nano::lmdb::store::pending ()
{
	return pending_store;
}

nano::unchecked_store & nano::lmdb::store::unchecked ()
{
	return unchecked_store;
}

nano::online_weight_store & nano::lmdb::store::online_weight ()
{
	return online_weight_store;
};

nano::pruned_store & nano::lmdb::store::pruned ()
{
	return pruned_store;
}

nano::peer_store & nano::lmdb::store::peer ()
{
	return peer_store;
}

nano::confirmation_height_store & nano::lmdb::store::confirmation_height ()
{
	return confirmation_height_store;
}

nano::final_vote_store & nano::lmdb::store::final_vote ()
{
	return final_vote_store;
}

nano::version_store & nano::lmdb::store::version ()
{
	return version_store;
}

rsnano::LmdbStoreHandle * nano::lmdb::store::get_handle () const
{
	return handle;
}
