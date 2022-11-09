#pragma once

#include <nano/lib/lmdbconfig.hpp>
#include <nano/lib/locks.hpp>
#include <nano/lib/work.hpp>
#include <nano/node/lmdb/lmdb.hpp>
#include <nano/node/lmdb/wallet_value.hpp>
#include <nano/node/openclwork.hpp>
#include <nano/secure/common.hpp>
#include <nano/secure/store.hpp>

#include <atomic>
#include <mutex>
#include <thread>
#include <unordered_set>
namespace rsnano
{
class KdfHandle;
}
namespace nano
{
class node;
class node_config;
class wallets;
class kdf final
{
public:
	kdf (unsigned kdf_work);
	kdf (kdf const &) = delete;
	kdf (kdf &&) = delete;
	~kdf ();
	void phs (nano::raw_key &, std::string const &, nano::uint256_union const &);
	rsnano::KdfHandle * handle;
};
enum class key_type
{
	not_a_type,
	unknown,
	adhoc,
	deterministic
};
class wallet_store final
{
public:
	wallet_store (bool &, nano::kdf &, nano::transaction &, nano::account, unsigned, std::string const &);
	wallet_store (bool &, nano::kdf &, nano::transaction &, nano::account, unsigned, std::string const &, std::string const &);
	~wallet_store ();
	wallet_store (wallet_store const &) = delete;
	bool is_open () const;
	void lock ();
	void password (nano::raw_key & password_a) const;
	void set_password (nano::raw_key const & password_a);
	std::vector<nano::account> accounts (nano::transaction const &);
	nano::uint256_union check (nano::transaction const &);
	bool rekey (nano::transaction const &, std::string const &);
	bool valid_password (nano::transaction const &);
	bool attempt_password (nano::transaction const &, std::string const &);
	void wallet_key (nano::raw_key &, nano::transaction const &);
	void seed (nano::raw_key &, nano::transaction const &);
	void seed_set (nano::transaction const &, nano::raw_key const &);
	nano::key_type key_type (nano::wallet_value const &);
	nano::public_key deterministic_insert (nano::transaction const &);
	nano::public_key deterministic_insert (nano::transaction const &, uint32_t const);
	nano::raw_key deterministic_key (nano::transaction const &, uint32_t);
	uint32_t deterministic_index_get (nano::transaction const &);
	void deterministic_index_set (nano::transaction const &, uint32_t);
	void deterministic_clear (nano::transaction const &);
	nano::uint256_union salt (nano::transaction const &);
	bool is_representative (nano::transaction const &);
	nano::account representative (nano::transaction const &);
	void representative_set (nano::transaction const &, nano::account const &);
	nano::public_key insert_adhoc (nano::transaction const &, nano::raw_key const &);
	bool insert_watch (nano::transaction const &, nano::account const &);
	void erase (nano::transaction const &, nano::account const &);
	bool fetch (nano::transaction const &, nano::account const &, nano::raw_key &);
	bool exists (nano::transaction const &, nano::account const &);
	void destroy (nano::transaction const &);
	nano::store_iterator<nano::account, nano::wallet_value> find (nano::transaction const &, nano::account const &);
	nano::store_iterator<nano::account, nano::wallet_value> begin (nano::transaction const &, nano::account const &);
	nano::store_iterator<nano::account, nano::wallet_value> begin (nano::transaction const &);
	nano::store_iterator<nano::account, nano::wallet_value> end ();
	void derive_key (nano::raw_key &, nano::transaction const &, std::string const &);
	void serialize_json (nano::transaction const &, std::string &);
	void write_backup (nano::transaction const &, boost::filesystem::path const &);
	bool move (nano::transaction const &, nano::wallet_store &, std::vector<nano::public_key> const &);
	bool import (nano::transaction const &, nano::wallet_store &);
	bool work_get (nano::transaction const &, nano::public_key const &, uint64_t &);
	void work_put (nano::transaction const &, nano::public_key const &, uint64_t);
	unsigned version (nano::transaction const &);
	static unsigned const version_4 = 4;
	static unsigned constexpr version_current = version_4;
	static int const special_count;
	nano::kdf & kdf;
	std::recursive_mutex mutex;
	unsigned fanout;

private:
	rsnano::LmdbWalletStoreHandle * rust_handle;
};
// A wallet is a set of account keys encrypted by a common encryption key
class wallet final : public std::enable_shared_from_this<nano::wallet>
{
public:
	std::shared_ptr<nano::block> change_action (nano::account const &, nano::account const &, uint64_t = 0, bool = true);
	std::shared_ptr<nano::block> receive_action (nano::block_hash const &, nano::account const &, nano::uint128_union const &, nano::account const &, uint64_t = 0, bool = true);
	std::shared_ptr<nano::block> send_action (nano::account const &, nano::account const &, nano::uint128_t const &, uint64_t = 0, bool = true, boost::optional<std::string> = {});
	bool action_complete (std::shared_ptr<nano::block> const &, nano::account const &, bool const, nano::block_details const &);
	wallet (bool &, nano::transaction &, nano::wallets &, std::string const &);
	wallet (bool &, nano::transaction &, nano::wallets &, std::string const &, std::string const &);
	void enter_initial_password ();
	bool enter_password (nano::transaction const &, std::string const &);
	nano::public_key insert_adhoc (nano::raw_key const &, bool = true);
	bool insert_watch (nano::transaction const &, nano::public_key const &);
	nano::public_key deterministic_insert (nano::transaction const &, bool = true);
	nano::public_key deterministic_insert (uint32_t, bool = true);
	nano::public_key deterministic_insert (bool = true);
	bool exists (nano::public_key const &);
	bool import (std::string const &, std::string const &);
	void serialize (std::string &);
	bool change_sync (nano::account const &, nano::account const &);
	void change_async (nano::account const &, nano::account const &, std::function<void (std::shared_ptr<nano::block> const &)> const &, uint64_t = 0, bool = true);
	bool receive_sync (std::shared_ptr<nano::block> const &, nano::account const &, nano::uint128_t const &);
	void receive_async (nano::block_hash const &, nano::account const &, nano::uint128_t const &, nano::account const &, std::function<void (std::shared_ptr<nano::block> const &)> const &, uint64_t = 0, bool = true);
	nano::block_hash send_sync (nano::account const &, nano::account const &, nano::uint128_t const &);
	void send_async (nano::account const &, nano::account const &, nano::uint128_t const &, std::function<void (std::shared_ptr<nano::block> const &)> const &, uint64_t = 0, bool = true, boost::optional<std::string> = {});
	void work_cache_blocking (nano::account const &, nano::root const &);
	void work_update (nano::transaction const &, nano::account const &, nano::root const &, uint64_t);
	// Schedule work generation after a few seconds
	void work_ensure (nano::account const &, nano::root const &);
	bool search_receivable (nano::transaction const &);
	uint32_t deterministic_check (nano::transaction const & transaction_a, uint32_t index);
	/** Changes the wallet seed and returns the first account */
	nano::public_key change_seed (nano::transaction const & transaction_a, nano::raw_key const & prv_a, uint32_t count = 0);
	void deterministic_restore (nano::transaction const & transaction_a);
	bool live ();

	nano::wallet_store store;
	nano::wallets & wallets;
	nano::mutex representatives_mutex;
	std::unordered_set<nano::account> representatives;
};

class wallet_representatives
{
public:
	uint64_t voting{ 0 }; // Number of representatives with at least the configured minimum voting weight
	bool half_principal{ false }; // has representatives with at least 50% of principal representative requirements
	std::unordered_set<nano::account> accounts; // Representatives with at least the configured minimum voting weight
	bool have_half_rep () const
	{
		return half_principal;
	}
	bool exists (nano::account const & rep_a) const
	{
		return accounts.count (rep_a) > 0;
	}
	void clear ()
	{
		voting = 0;
		half_principal = false;
		accounts.clear ();
	}
};

/**
 * The wallets set is all the wallets a node controls.
 * A node may contain multiple wallets independently encrypted and operated.
 */
class wallets final
{
public:
	wallets (bool, nano::node &);
	~wallets ();
	std::shared_ptr<nano::wallet> open (nano::wallet_id const &);
	std::shared_ptr<nano::wallet> create (nano::wallet_id const &);
	bool search_receivable (nano::wallet_id const &);
	void search_receivable_all ();
	void destroy (nano::wallet_id const &);
	void reload ();
	void do_wallet_actions ();
	void queue_wallet_action (nano::uint128_t const &, std::shared_ptr<nano::wallet> const &, std::function<void (nano::wallet &)>);
	void foreach_representative (std::function<void (nano::public_key const &, nano::raw_key const &)> const &);
	bool exists (nano::transaction const &, nano::account const &);
	void start ();
	void stop ();
	void clear_send_ids (nano::transaction const &);
	nano::wallet_representatives reps () const;
	bool check_rep (nano::account const &, nano::uint128_t const &, bool const = true);
	void compute_reps ();
	void ongoing_compute_reps ();
	std::vector<nano::wallet_id> get_wallet_ids (nano::transaction const & transaction_a);
	std::unordered_map<nano::wallet_id, std::shared_ptr<nano::wallet>> get_wallets ();
	nano::block_hash get_block_hash (bool & error_a, nano::transaction const & transaction_a, std::string const & id_a);
	bool set_block_hash (nano::transaction const & transaction_a, std::string const & id_a, nano::block_hash const & hash);
	nano::network_params & network_params;
	std::function<void (bool)> observer;
	std::unordered_map<nano::wallet_id, std::shared_ptr<nano::wallet>> items;
	std::multimap<nano::uint128_t, std::pair<std::shared_ptr<nano::wallet>, std::function<void (nano::wallet &)>>, std::greater<nano::uint128_t>> actions;
	nano::locked<std::unordered_map<nano::account, nano::root>> delayed_work;
	nano::mutex mutex;
	nano::mutex action_mutex;
	nano::condition_variable condition;
	nano::kdf kdf;
	nano::node & node;
	nano::mdb_env & env;
	std::atomic<bool> stopped;
	std::thread thread;
	static nano::uint128_t const generate_priority;
	static nano::uint128_t const high_priority;
	/** Start read-write transaction */
	std::unique_ptr<nano::write_transaction> tx_begin_write ();

	/** Start read-only transaction */
	std::unique_ptr<nano::read_transaction> tx_begin_read ();

private:
	mutable nano::mutex reps_cache_mutex;
	nano::wallet_representatives representatives;

public:
	rsnano::LmdbWalletsHandle * rust_handle;
};

std::unique_ptr<container_info_component> collect_container_info (wallets & wallets, std::string const & name);

class wallets_store
{
public:
	virtual ~wallets_store () = default;
	virtual bool init_error () const = 0;
};
class mdb_wallets_store final : public wallets_store
{
public:
	mdb_wallets_store (boost::filesystem::path const &, nano::lmdb_config const & lmdb_config_a = nano::lmdb_config{});
	nano::mdb_env environment;
	bool init_error () const override;
	bool error{ false };
};
}
