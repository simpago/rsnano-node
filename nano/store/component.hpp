#pragma once

#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/memory.hpp>
#include <nano/lib/stream.hpp>
#include <nano/secure/common.hpp>
#include <nano/store/tables.hpp>
#include <nano/store/transaction.hpp>

#include <boost/endian/conversion.hpp>
#include <boost/polymorphic_cast.hpp>

#include <stack>

namespace nano
{
namespace store
{
	class account;
	class block;
	class confirmation_height;
	class final_vote;
	class online_weight;
	class peer;
	class pending;
	class pruned;
	class version;
}

namespace store
{
	/**
	 * Store manager
	 */
	class component
	{
	public:
		virtual ~component () = default;
		virtual store::block & block () = 0;
		virtual store::account & account () = 0;
		virtual store::pending & pending () = 0;
		virtual store::online_weight & online_weight () = 0;
		virtual store::pruned & pruned () = 0;
		virtual store::peer & peer () = 0;
		virtual store::confirmation_height & confirmation_height () = 0;
		virtual store::final_vote & final_vote () = 0;
		virtual store::version & version () = 0;
		static int constexpr version_minimum{ 21 };
		static int constexpr version_current{ 22 };

		virtual unsigned max_block_write_batch_num () const = 0;

		virtual bool copy_db (std::filesystem::path const & destination) = 0;
		virtual void rebuild_db (write_transaction const & transaction_a) = 0;

		/** Not applicable to all sub-classes */
		virtual void serialize_mdb_tracker (boost::property_tree::ptree &, std::chrono::milliseconds, std::chrono::milliseconds) {};
		virtual void serialize_memory_stats (boost::property_tree::ptree &) = 0;

		virtual bool init_error () const = 0;

		/** Start read-write transaction */
		virtual std::unique_ptr<nano::store::write_transaction> tx_begin_write (std::vector<nano::tables> const & tables_to_lock = {}, std::vector<nano::tables> const & tables_no_lock = {}) = 0;

		/** Start read-only transaction */
		virtual std::unique_ptr<nano::store::read_transaction> tx_begin_read () const = 0;

		virtual std::string vendor_get () const = 0;
		virtual rsnano::LmdbStoreHandle * get_handle () const = 0;
	};
} // namespace store
} // namespace nano
