#include "nano/store/lmdb/lmdb_env.hpp"

#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/election.hpp>
#include <nano/node/node.hpp>
#include <nano/node/wallet.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/store/lmdb/iterator.hpp>

#include <boost/format.hpp>
#include <boost/polymorphic_cast.hpp>
#include <boost/property_tree/json_parser.hpp>

#include <algorithm>
#include <cstdint>
#include <memory>
#include <stdexcept>

nano::uint256_union nano::wallet_store::check (store::transaction const & transaction_a)
{
	nano::uint256_union result;
	rsnano::rsn_lmdb_wallet_store_check (rust_handle, transaction_a.get_rust_handle (), result.bytes.data ());
	return result;
}

nano::uint256_union nano::wallet_store::salt (store::transaction const & transaction_a)
{
	nano::uint256_union result;
	rsnano::rsn_lmdb_wallet_store_salt (rust_handle, transaction_a.get_rust_handle (), result.bytes.data ());
	return result;
}

void nano::wallet_store::wallet_key (nano::raw_key & prv_a, store::transaction const & transaction_a)
{
	rsnano::rsn_lmdb_wallet_store_wallet_key (rust_handle, prv_a.bytes.data (), transaction_a.get_rust_handle ());
}

void nano::wallet_store::seed (nano::raw_key & prv_a, store::transaction const & transaction_a)
{
	rsnano::rsn_lmdb_wallet_store_seed (rust_handle, prv_a.bytes.data (), transaction_a.get_rust_handle ());
}

void nano::wallet_store::seed_set (store::transaction const & transaction_a, nano::raw_key const & prv_a)
{
	rsnano::rsn_lmdb_wallet_store_seed_set (rust_handle, transaction_a.get_rust_handle (), prv_a.bytes.data ());
}

nano::public_key nano::wallet_store::deterministic_insert (store::transaction const & transaction_a)
{
	nano::public_key key;
	rsnano::rsn_lmdb_wallet_store_deterministic_insert (rust_handle, transaction_a.get_rust_handle (), key.bytes.data ());
	return key;
}

nano::raw_key nano::wallet_store::deterministic_key (store::transaction const & transaction_a, uint32_t index_a)
{
	nano::raw_key key;
	rsnano::rsn_lmdb_wallet_store_deterministic_key (rust_handle, transaction_a.get_rust_handle (), index_a, key.bytes.data ());
	return key;
}

void nano::wallet_store::password (nano::raw_key & password_a) const
{
	rsnano::rsn_lmdb_wallet_store_password (rust_handle, password_a.bytes.data ());
}

uint32_t nano::wallet_store::deterministic_index_get (store::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_wallet_store_deterministic_index_get (rust_handle, transaction_a.get_rust_handle ());
}

void nano::wallet_store::deterministic_index_set (store::transaction const & transaction_a, uint32_t index_a)
{
	rsnano::rsn_lmdb_wallet_store_deterministic_index_set (rust_handle, transaction_a.get_rust_handle (), index_a);
}

void nano::wallet_store::deterministic_clear (store::transaction const & transaction_a)
{
	rsnano::rsn_lmdb_wallet_store_deterministic_clear (rust_handle, transaction_a.get_rust_handle ());
}

bool nano::wallet_store::valid_password (store::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_wallet_store_valid_password (rust_handle, transaction_a.get_rust_handle ());
}

bool nano::wallet_store::attempt_password (store::transaction const & transaction_a, std::string const & password_a)
{
	return !rsnano::rsn_lmdb_wallet_store_attempt_password (rust_handle, transaction_a.get_rust_handle (), password_a.c_str ());
}

bool nano::wallet_store::rekey (store::transaction const & transaction_a, std::string const & password_a)
{
	return !rsnano::rsn_lmdb_wallet_store_rekey (rust_handle, transaction_a.get_rust_handle (), password_a.c_str ());
}

void nano::wallet_store::derive_key (nano::raw_key & prv_a, store::transaction const & transaction_a, std::string const & password_a)
{
	rsnano::rsn_lmdb_wallet_store_derive_key (rust_handle, prv_a.bytes.data (), transaction_a.get_rust_handle (), password_a.c_str ());
}

int const nano::wallet_store::special_count (7);

nano::wallet_store::wallet_store (bool & init_a, nano::kdf & kdf_a, store::transaction & transaction_a, nano::account representative_a, unsigned fanout_a, std::string const & wallet_a, std::string const & json_a) :
	rust_handle{ rsnano::rsn_lmdb_wallet_store_create2 (fanout_a, kdf_a.handle, transaction_a.get_rust_handle (), wallet_a.c_str (), json_a.c_str ()) }
{
	init_a = rust_handle == nullptr;
}

nano::wallet_store::wallet_store (bool & init_a, nano::kdf & kdf_a, store::transaction & transaction_a, nano::account representative_a, unsigned fanout_a, std::string const & wallet_a) :
	rust_handle{ rsnano::rsn_lmdb_wallet_store_create (fanout_a, kdf_a.handle, transaction_a.get_rust_handle (), representative_a.bytes.data (), wallet_a.c_str ()) }
{
	init_a = rust_handle == nullptr;
}

nano::wallet_store::wallet_store (rsnano::LmdbWalletStoreHandle * handle) :
	rust_handle{ handle }
{
}

nano::wallet_store::~wallet_store ()
{
	if (rust_handle != nullptr)
		rsnano::rsn_lmdb_wallet_store_destroy (rust_handle);
}

void nano::wallet_store::set_password (nano::raw_key const & password_a)
{
	rsnano::rsn_lmdb_wallet_store_set_password (rust_handle, password_a.bytes.data ());
}

bool nano::wallet_store::is_representative (store::transaction const & transaction_a)
{
	return exists (transaction_a, representative (transaction_a));
}

void nano::wallet_store::representative_set (store::transaction const & transaction_a, nano::account const & representative_a)
{
	rsnano::rsn_lmdb_wallet_store_representative_set (rust_handle, transaction_a.get_rust_handle (), representative_a.bytes.data ());
}

nano::account nano::wallet_store::representative (store::transaction const & transaction_a)
{
	nano::account rep;
	rsnano::rsn_lmdb_wallet_store_representative (rust_handle, transaction_a.get_rust_handle (), rep.bytes.data ());
	return rep;
}

nano::public_key nano::wallet_store::insert_adhoc (store::transaction const & transaction_a, nano::raw_key const & prv)
{
	nano::public_key pub;
	rsnano::rsn_lmdb_wallet_store_insert_adhoc (rust_handle, transaction_a.get_rust_handle (), prv.bytes.data (), pub.bytes.data ());
	return pub;
}

bool nano::wallet_store::insert_watch (store::transaction const & transaction_a, nano::account const & pub_a)
{
	return !rsnano::rsn_lmdb_wallet_store_insert_watch (rust_handle, transaction_a.get_rust_handle (), pub_a.bytes.data ());
}

void nano::wallet_store::erase (store::transaction const & transaction_a, nano::account const & pub)
{
	rsnano::rsn_lmdb_wallet_store_erase (rust_handle, transaction_a.get_rust_handle (), pub.bytes.data ());
}

bool nano::wallet_store::fetch (store::transaction const & transaction_a, nano::account const & pub, nano::raw_key & prv)
{
	return !rsnano::rsn_lmdb_wallet_store_fetch (rust_handle, transaction_a.get_rust_handle (), pub.bytes.data (), prv.bytes.data ());
}

bool nano::wallet_store::exists (store::transaction const & transaction_a, nano::public_key const & pub)
{
	return rsnano::rsn_lmdb_wallet_store_exists (rust_handle, transaction_a.get_rust_handle (), pub.bytes.data ());
}

void nano::wallet_store::serialize_json (store::transaction const & transaction_a, std::string & string_a)
{
	rsnano::StringDto dto;
	rsnano::rsn_lmdb_wallet_store_serialize_json (rust_handle, transaction_a.get_rust_handle (), &dto);
	string_a = rsnano::convert_dto_to_string (dto);
}

bool nano::wallet_store::move (store::transaction const & transaction_a, nano::wallet_store & other_a, std::vector<nano::public_key> const & keys)
{
	return !rsnano::rsn_lmdb_wallet_store_move (rust_handle, transaction_a.get_rust_handle (), other_a.rust_handle, reinterpret_cast<const uint8_t *> (keys.data ()), keys.size ());
}

nano::kdf::kdf (unsigned kdf_work) :
	handle{ rsnano::rsn_kdf_create (kdf_work) }
{
}

nano::kdf::~kdf ()
{
	rsnano::rsn_kdf_destroy (handle);
}

void nano::kdf::phs (nano::raw_key & result_a, std::string const & password_a, nano::uint256_union const & salt_a)
{
	rsnano::rsn_kdf_phs (handle, result_a.bytes.data (), password_a.c_str (), salt_a.bytes.data ());
}

namespace
{
void wrapped_wallet_action_callback (void * context, rsnano::WalletHandle * wallet_handle)
{
	auto action = static_cast<std::function<void (nano::wallet &)> *> (context);
	auto wallet{ std::make_shared<nano::wallet> (wallet_handle) };
	(*action) (*wallet);
}

void delete_wallet_action_context (void * context)
{
	auto action = static_cast<std::function<void (nano::wallet &)> *> (context);
	delete action;
}
}

namespace
{
void wrapped_wallet_action_observer (void * context, bool active)
{
	auto callback = static_cast<std::function<void (bool)> *> (context);
	(*callback) (active);
}

void delete_wallet_action_observer_context (void * context)
{
	auto callback = static_cast<std::function<void (bool)> *> (context);
	delete callback;
}

void start_election_wrapper (void * context, rsnano::BlockHandle * block_handle)
{
	auto callback = static_cast<std::function<void (std::shared_ptr<nano::block> const &)> *> (context);
	auto block{ nano::block_handle_to_block (block_handle) };
	(*callback) (block);
}

void delete_start_election_context (void * context)
{
	auto callback = static_cast<std::function<void (std::shared_ptr<nano::block> const &)> *> (context);
	delete callback;
}
}

nano::wallet::representatives_lock::representatives_lock (rsnano::RepresentativesLockHandle * handle) :
	handle{ handle }
{
}

nano::wallet::representatives_lock::~representatives_lock ()
{
	rsnano::rsn_representatives_lock_destroy (handle);
}

size_t nano::wallet::representatives_lock::size () const
{
	return rsnano::rsn_representatives_lock_size (handle);
}

void nano::wallet::representatives_lock::insert (nano::public_key const & rep)
{
	rsnano::rsn_representatives_lock_insert (handle, rep.bytes.data ());
}

std::unordered_set<nano::account> nano::wallet::representatives_lock::get_all ()
{
	auto vec_handle = rsnano::rsn_representatives_lock_get_all (handle);
	std::unordered_set<nano::account> result{};
	auto len = rsnano::rsn_account_vec_len (vec_handle);
	for (auto i = 0; i < len; ++i)
	{
		nano::account rep;
		rsnano::rsn_account_vec_get (vec_handle, i, rep.bytes.data ());
		result.insert (rep);
	}
	rsnano::rsn_account_vec_destroy (vec_handle);
	return result;
}

void nano::wallet::representatives_lock::set (std::unordered_set<nano::account> reps)
{
	rsnano::rsn_representatives_lock_clear (handle);
	for (const auto & rep : reps)
	{
		insert (rep);
	}
}

nano::wallet::representatives_mutex::representatives_mutex (rsnano::WalletHandle * handle) :
	handle{ handle }
{
}

nano::wallet::representatives_lock nano::wallet::representatives_mutex::lock ()
{
	return { rsnano::rsn_representatives_lock_create (handle) };
}

nano::wallet::wallet (rsnano::WalletHandle * handle) :
	store{ rsnano::rsn_wallet_store (handle) },
	handle{ handle },
	representatives_mutex{ handle }
{
}

nano::wallet::~wallet ()
{
	if (handle != nullptr)
		rsnano::rsn_wallet_destroy (handle);
}

size_t nano::wallet::representatives_count () const
{
	auto representatives_lk (representatives_mutex.lock ());
	return representatives_lk.size ();
}

//---------------------------------------
nano::wallet_representatives_lock::wallet_representatives_lock (rsnano::WalletRepresentativesLock * handle) :
	handle{ handle }
{
}

nano::wallet_representatives_lock::wallet_representatives_lock (nano::wallet_representatives_lock && other)
{
	handle = other.handle;
	other.handle = nullptr;
}

nano::wallet_representatives_lock::~wallet_representatives_lock ()
{
	if (handle != nullptr)
	{
		rsnano::rsn_wallet_representatives_lock_destroy (handle);
	}
}

uint64_t nano::wallet_representatives_lock::voting_reps () const
{
	return rsnano::rsn_wallet_representatives_lock_voting_reps (handle);
}

bool nano::wallet_representatives_lock::have_half_rep () const
{
	return rsnano::rsn_wallet_representatives_lock_have_half_rep (handle);
}

bool nano::wallet_representatives_lock::exists (nano::account const & rep_a) const
{
	return rsnano::rsn_wallet_representatives_lock_exists (handle, rep_a.bytes.data ());
}

void nano::wallet_representatives_lock::clear ()
{
	return rsnano::rsn_wallet_representatives_lock_clear (handle);
}

bool nano::wallet_representatives_lock::check_rep (nano::account const & account_a, nano::uint128_t const & half_principal_weight_a)
{
	nano::amount half_weight{ half_principal_weight_a };
	return rsnano::rsn_wallet_representatives_lock_check_rep (handle, account_a.bytes.data (), half_weight.bytes.data ());
}

//---------------------------------------

nano::wallets::wallets_mutex_lock::wallets_mutex_lock (rsnano::WalletsMutexLockHandle * handle) :
	handle{ handle }
{
}

nano::wallets::wallets_mutex_lock::wallets_mutex_lock (nano::wallets::wallets_mutex_lock && other) :
	handle{ other.handle }
{
	other.handle = nullptr;
}

nano::wallets::wallets_mutex_lock::~wallets_mutex_lock ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_wallets_mutex_lock_destroy (handle);
}

std::shared_ptr<nano::wallet> nano::wallets::wallets_mutex_lock::find (nano::wallet_id const & wallet_id)
{
	rsnano::WalletHandle * wallet_handle = nullptr;
	std::shared_ptr<nano::wallet> wallet{};
	if (rsnano::rsn_lmdb_wallets_mutex_lock_find (handle, wallet_id.bytes.data (), &wallet_handle))
	{
		wallet = make_shared<nano::wallet> (wallet_handle);
	}
	return wallet;
}

std::unordered_map<nano::wallet_id, std::shared_ptr<nano::wallet>> nano::wallets::wallets_mutex_lock::get_all ()
{
	auto vec_handle = rsnano::rsn_lmdb_wallets_mutex_lock_get_all (handle);
	std::unordered_map<nano::wallet_id, std::shared_ptr<nano::wallet>> result;
	auto len = rsnano::rsn_wallet_vec_len (vec_handle);
	for (auto i = 0; i < len; ++i)
	{
		nano::wallet_id id;
		auto wallet_handle = rsnano::rsn_wallet_vec_get (vec_handle, i, id.bytes.data ());
		auto wallet{ std::make_shared<nano::wallet> (wallet_handle) };
		result.emplace (id, wallet);
	}
	rsnano::rsn_wallet_vec_destroy (vec_handle);
	return result;
}

size_t nano::wallets::wallets_mutex_lock::size () const
{
	return rsnano::rsn_lmdb_wallets_mutex_lock_size (handle);
}

nano::wallets::wallets_mutex::wallets_mutex (rsnano::LmdbWalletsHandle * handle) :
	handle{ handle }
{
}

nano::wallets::wallets_mutex_lock nano::wallets::wallets_mutex::lock ()
{
	return { rsnano::rsn_lmdb_wallets_mutex_lock (handle) };
}

boost::optional<nano::wallets::wallets_mutex_lock> nano::wallets::wallets_mutex::try_lock ()
{
	auto lock_handle = rsnano::rsn_lmdb_wallets_mutex_try_lock (handle);
	if (lock_handle != nullptr)
	{
		return { nano::wallets::wallets_mutex_lock{ lock_handle } };
	}
	return {};
}

namespace
{
rsnano::LmdbWalletsHandle * create_wallets (nano::node & node_a)
{
	auto config_dto{ node_a.config->to_dto () };
	auto network_params_dto{ node_a.network_params.to_dto () };

	return rsnano::rsn_lmdb_wallets_create (
	node_a.config->enable_voting,
	node_a.application_path.c_str (),
	node_a.ledger.handle,
	&config_dto,
	node_a.config->network_params.kdf_work,
	&node_a.config->network_params.work.dto,
	node_a.distributed_work.handle,
	&network_params_dto,
	node_a.workers->handle,
	node_a.block_processor.handle,
	node_a.representative_register.handle,
	node_a.network->tcp_channels->handle,
	node_a.confirming_set.handle);
}
}

nano::wallets::wallets (nano::node & node_a) :
	rust_handle{ create_wallets (node_a) },
	mutex{ rust_handle }
{
}

nano::wallets::wallets (rsnano::LmdbWalletsHandle * handle) :
	rust_handle{ handle },
	mutex{ handle }
{
}

nano::wallets::~wallets ()
{
	stop_actions ();
	rsnano::rsn_lmdb_wallets_destroy (rust_handle);
}

size_t nano::wallets::wallet_count () const
{
	auto lock{ mutex.lock () };
	return lock.size ();
}

size_t nano::wallets::representatives_count (nano::wallet_id const & id) const
{
	auto lock{ mutex.lock () };
	auto wallet = lock.find (id);
	return wallet->representatives_count ();
}

nano::key_type nano::wallets::key_type (nano::wallet_id const & wallet_id, nano::account const & account)
{
	return static_cast<nano::key_type> (rsnano::rsn_wallets_key_type (rust_handle, wallet_id.bytes.data (), account.bytes.data ()));
}

nano::wallets_error nano::wallets::get_representative (nano::wallet_id const & wallet_id, nano::account & representative)
{
	return static_cast<nano::wallets_error> (rsnano::rsn_wallets_get_representative (rust_handle, wallet_id.bytes.data (), representative.bytes.data ()));
}

nano::wallets_error nano::wallets::set_representative (nano::wallet_id const & wallet_id, nano::account const & rep, bool update_existing_accounts)
{
	auto result = rsnano::rsn_wallets_set_representative (rust_handle, wallet_id.bytes.data (), rep.bytes.data (), update_existing_accounts);
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::get_seed (nano::wallet_id const & wallet_id, nano::raw_key & prv_a) const
{
	auto result = rsnano::rsn_wallets_get_seed (rust_handle, wallet_id.bytes.data (), prv_a.bytes.data ());
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::change_seed (nano::wallet_id const & wallet_id, nano::raw_key const & prv_a, uint32_t count, nano::public_key & first_account, uint32_t & restored_count)
{
	auto result = rsnano::rsn_wallets_change_seed2 (rust_handle, wallet_id.bytes.data (), prv_a.bytes.data (), count, first_account.bytes.data (), &restored_count);
	return static_cast<nano::wallets_error> (result);
}

bool nano::wallets::ensure_wallet_is_unlocked (nano::wallet_id const & wallet_id, std::string const & password_a)
{
	return rsnano::rsn_wallets_ensure_wallet_is_unlocked (rust_handle, wallet_id.bytes.data (), password_a.c_str ());
}

bool nano::wallets::import_replace (nano::wallet_id const & wallet_id, std::string const & json_a, std::string const & password_a)
{
	return rsnano::rsn_wallets_import_replace (rust_handle, wallet_id.bytes.data (), json_a.c_str (), password_a.c_str ());
}

bool nano::wallets::import (nano::wallet_id const & wallet_id, std::string const & json_a)
{
	return rsn_wallets_import (rust_handle, wallet_id.bytes.data (), json_a.c_str ());
}

nano::wallets_error nano::wallets::decrypt (nano::wallet_id const & wallet_id, std::vector<std::pair<nano::account, nano::raw_key>> & accounts) const
{
	uint8_t error;
	auto result_handle = rsnano::rsn_wallets_decrypt (rust_handle, wallet_id.bytes.data (), &error);
	auto result = static_cast<nano::wallets_error> (error);
	if (result_handle != nullptr)
	{
		auto len = rsnano::rsn_decrypt_result_len (result_handle);
		for (auto i = 0; i < len; ++i)
		{
			nano::account acc;
			nano::raw_key key;
			rsnano::rsn_decrypt_result_get (result_handle, i, acc.bytes.data (), key.bytes.data ());
			accounts.emplace_back (acc, key);
		}
		rsnano::rsn_decrypt_result_destroy (result_handle);
	}
	return result;
}

nano::wallets_error nano::wallets::fetch (nano::wallet_id const & wallet_id, nano::account const & pub, nano::raw_key & prv)
{
	auto result = rsnano::rsn_wallets_fetch (rust_handle, wallet_id.bytes.data (), pub.bytes.data (), prv.bytes.data ());
	return static_cast<nano::wallets_error> (result);
}

std::vector<nano::wallet_id> nano::wallets::get_wallet_ids () const
{
	auto lock{ mutex.lock () };
	std::vector<nano::wallet_id> result{};
	result.reserve (lock.size ());
	auto wallets{ lock.get_all () };
	for (auto i (wallets.begin ()), n (wallets.end ()); i != n; ++i)
	{
		result.push_back (i->first);
	}
	return result;
}

nano::wallets_error nano::wallets::get_accounts (nano::wallet_id const & wallet_id, std::vector<nano::account> & accounts)
{
	uint8_t error_code;
	auto vec_handle = rsnano::rsn_wallets_get_accounts_of_wallet (rust_handle, wallet_id.bytes.data (), &error_code);
	auto error = static_cast<nano::wallets_error> (error_code);
	if (error == nano::wallets_error::none)
	{
		rsnano::account_vec acc_vec{ vec_handle };
		accounts = acc_vec.into_vector ();
	}
	return error;
}

nano::wallets_error nano::wallets::work_get (nano::wallet_id const & wallet_id, nano::account const & account, uint64_t & work)
{
	auto result = rsnano::rsn_wallets_work_get2 (rust_handle, wallet_id.bytes.data (), account.bytes.data (), &work);
	return static_cast<nano::wallets_error> (result);
}

uint64_t nano::wallets::work_get (nano::wallet_id const & wallet_id, nano::account const & account)
{
	return rsnano::rsn_wallets_work_get (rust_handle, wallet_id.bytes.data (), account.bytes.data ());
}

nano::wallets_error nano::wallets::work_set (nano::wallet_id const & wallet_id, nano::account const & account, uint64_t work)
{
	auto result = rsnano::rsn_wallets_work_set (rust_handle, wallet_id.bytes.data (), account.bytes.data (), work);
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::remove_account (nano::wallet_id const & wallet_id, nano::account const & account_id)
{
	auto result = rsnano::rsn_wallets_remove_account (rust_handle, wallet_id.bytes.data (), account_id.bytes.data ());
	return static_cast<nano::wallets_error> (result);
}

bool nano::wallets::move_accounts (nano::wallet_id const & source_id, nano::wallet_id const & target_id, std::vector<nano::public_key> const & accounts)
{
	rsnano::account_vec acc_vec{ accounts };
	auto result = rsnano::rsn_wallets_move_accounts (rust_handle, source_id.bytes.data (), target_id.bytes.data (), acc_vec.handle);
	return result != 0;
}

bool nano::wallets::wallet_exists (nano::wallet_id const & id) const
{
	auto lock{ mutex.lock () };
	return lock.find (id) != nullptr;
}

nano::wallet_id nano::wallets::first_wallet_id () const
{
	auto lock{ mutex.lock () };
	auto wallets{ lock.get_all () };
	return wallets.begin ()->first;
}

nano::wallets_error nano::wallets::insert_adhoc (nano::wallet_id const & wallet_id, nano::raw_key const & key_a, bool generate_work_a)
{
	nano::account account;
	return static_cast<nano::wallets_error> (rsnano::rsn_wallets_insert_adhoc2 (rust_handle, wallet_id.bytes.data (), key_a.bytes.data (), generate_work_a, account.bytes.data ()));
}

nano::wallets_error nano::wallets::insert_adhoc (nano::wallet_id const & wallet_id, nano::raw_key const & key_a, bool generate_work_a, nano::public_key & account)
{
	return static_cast<nano::wallets_error> (rsnano::rsn_wallets_insert_adhoc2 (rust_handle, wallet_id.bytes.data (), key_a.bytes.data (), generate_work_a, account.bytes.data ()));
}

std::shared_ptr<nano::block> nano::wallets::receive_action (const std::shared_ptr<nano::wallet> & wallet, nano::block_hash const & send_hash_a, nano::account const & representative_a, nano::uint128_union const & amount_a, nano::account const & account_a, uint64_t work_a, bool generate_work_a)
{
	auto block_handle = rsnano::rsn_wallets_receive_action (rust_handle, wallet->handle, send_hash_a.bytes.data (), representative_a.bytes.data (), amount_a.bytes.data (), account_a.bytes.data (), work_a, generate_work_a);
	return nano::block_handle_to_block (block_handle);
}

std::shared_ptr<nano::block> nano::wallets::change_action (const std::shared_ptr<wallet> & wallet, nano::account const & source_a, nano::account const & representative_a, uint64_t work_a, bool generate_work_a)
{
	auto block_handle = rsnano::rsn_wallets_change_action (rust_handle, wallet->handle, source_a.bytes.data (), representative_a.bytes.data (), work_a, generate_work_a);
	return nano::block_handle_to_block (block_handle);
}

std::shared_ptr<nano::block> nano::wallets::send_action (const std::shared_ptr<nano::wallet> & wallet, nano::account const & source_a, nano::account const & account_a, nano::uint128_t const & amount_a, uint64_t work_a, bool generate_work_a, boost::optional<std::string> id_a)
{
	nano::amount amount{ amount_a };
	const char * id_ptr = nullptr;
	if (id_a.has_value ())
	{
		id_ptr = id_a.value ().c_str ();
	}
	auto block_handle = rsnano::rsn_wallets_send_action (rust_handle, wallet->handle, source_a.bytes.data (), account_a.bytes.data (), amount.bytes.data (), work_a, generate_work_a, id_ptr);
	return nano::block_handle_to_block (block_handle);
}

bool nano::wallets::change_sync (const std::shared_ptr<nano::wallet> & wallet, nano::account const & source_a, nano::account const & representative_a)
{
	return rsnano::rsn_wallets_change_sync_wallet (rust_handle, wallet->handle, source_a.bytes.data (), representative_a.bytes.data ());
}

bool nano::wallets::receive_sync (const std::shared_ptr<nano::wallet> & wallet, std::shared_ptr<nano::block> const & block_a, nano::account const & representative_a, nano::uint128_t const & amount_a)
{
	nano::amount amount{ amount_a };
	return rsnano::rsn_wallets_receive_sync (rust_handle, wallet->handle, block_a->get_handle (), representative_a.bytes.data (), amount.bytes.data ());
}

void nano::wallets::enter_initial_password (const std::shared_ptr<nano::wallet> & wallet)
{
	rsnano::rsn_wallets_enter_initial_password (rust_handle, wallet->handle);
}

nano::root nano::wallets::get_delayed_work (nano::account const & account)
{
	nano::root result;
	rsnano::rsn_wallets_get_delayed_work (rust_handle, account.bytes.data (), result.bytes.data ());
	return result;
}

void nano::wallets::stop_actions ()
{
	rsnano::rsn_wallets_stop (rust_handle);
}

nano::wallet_representatives_lock nano::wallets::lock_representatives () const
{
	return { rsnano::rsn_wallets_representatives_lock (rust_handle) };
}

nano::wallets_error nano::wallets::insert_watch (nano::wallet_id const & wallet_id, std::vector<nano::public_key> const & accounts)
{
	rsnano::account_vec account_vec{ accounts };
	auto result = rsnano::rsn_wallets_insert_watch (rust_handle, wallet_id.bytes.data (), account_vec.handle);
	return static_cast<nano::wallets_error> (result);
}

void nano::wallets::set_password (nano::wallet_id const & wallet_id, nano::raw_key const & password)
{
	auto lock{ mutex.lock () };
	auto wallet{ lock.find (wallet_id) };
	wallet->store.set_password (password);
}

void nano::wallets::password (nano::wallet_id const & wallet_id, nano::raw_key & password_a) const
{
	auto lock{ mutex.lock () };
	auto wallet{ lock.find (wallet_id) };
	wallet->store.password (password_a);
}

nano::wallets_error nano::wallets::enter_password (nano::wallet_id const & wallet_id, std::string const & password_a)
{
	return static_cast<nano::wallets_error> (rsnano::rsn_wallets_enter_password2 (rust_handle, wallet_id.bytes.data (), password_a.c_str ()));
}

void nano::wallets::enter_initial_password (nano::wallet_id const & wallet_id)
{
	auto lock{ mutex.lock () };
	auto wallet{ lock.find (wallet_id) };
	enter_initial_password (wallet);
}

nano::wallets_error nano::wallets::valid_password (nano::wallet_id const & wallet_id, bool & valid)
{
	auto error = rsnano::rsn_wallets_valid_password (rust_handle, wallet_id.bytes.data (), &valid);
	return static_cast<nano::wallets_error> (error);
}

nano::wallets_error nano::wallets::attempt_password (nano::wallet_id const & wallet_id, std::string const & password)
{
	auto error = rsnano::rsn_wallets_attempt_password (rust_handle, wallet_id.bytes.data (), password.c_str ());
	return static_cast<nano::wallets_error> (error);
}

nano::wallets_error nano::wallets::rekey (nano::wallet_id const wallet_id, std::string const & password)
{
	auto error = rsnano::rsn_wallets_rekey (rust_handle, wallet_id.bytes.data (), password.c_str ());
	return static_cast<nano::wallets_error> (error);
}

nano::wallets_error nano::wallets::lock (nano::wallet_id const & wallet_id)
{
	auto error = rsnano::rsn_wallets_lock (rust_handle, wallet_id.bytes.data ());
	return static_cast<nano::wallets_error> (error);
}

nano::wallets_error nano::wallets::deterministic_insert (nano::wallet_id const & wallet_id, uint32_t const index, bool generate_work_a, nano::account & account)
{
	auto result = rsnano::rsn_wallets_deterministic_insert2 (rust_handle, wallet_id.bytes.data (), index, generate_work_a, account.bytes.data ());
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::deterministic_insert (nano::wallet_id const & wallet_id, bool generate_work_a, nano::account & account)
{
	auto result = rsnano::rsn_wallets_deterministic_insert3 (rust_handle, wallet_id.bytes.data (), generate_work_a, account.bytes.data ());
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::deterministic_index_get (nano::wallet_id const & wallet_id, uint32_t & index)
{
	auto result = rsnano::rsn_wallets_deterministic_index_get (rust_handle, wallet_id.bytes.data (), &index);
	return static_cast<nano::wallets_error> (result);
}

void nano::wallets::work_cache_blocking (nano::wallet_id const & wallet_id, nano::account const & account_a, nano::root const & root_a)
{
	rsnano::rsn_wallets_work_cache_blocking (rust_handle, wallet_id.bytes.data (), account_a.bytes.data (), root_a.bytes.data ());
}

std::shared_ptr<nano::block> nano::wallets::send_action (nano::wallet_id const & wallet_id, nano::account const & source_a, nano::account const & account_a, nano::uint128_t const & amount_a, uint64_t work_a, bool generate_work_a, boost::optional<std::string> id_a)
{
	auto lock{ mutex.lock () };
	auto wallet = lock.find (wallet_id);
	return send_action (wallet, source_a, account_a, amount_a, work_a, generate_work_a, id_a);
}

std::shared_ptr<nano::block> nano::wallets::receive_action (nano::wallet_id const & wallet_id, nano::block_hash const & send_hash_a, nano::account const & representative_a, nano::uint128_union const & amount_a, nano::account const & account_a, uint64_t work_a, bool generate_work_a)
{
	auto lock{ mutex.lock () };
	auto wallet = lock.find (wallet_id);
	return receive_action (wallet, send_hash_a, representative_a, amount_a, account_a, work_a, generate_work_a);
}

std::shared_ptr<nano::block> nano::wallets::change_action (nano::wallet_id const & wallet_id, nano::account const & source_a, nano::account const & representative_a, uint64_t work_a, bool generate_work_a)
{
	auto lock{ mutex.lock () };
	auto wallet = lock.find (wallet_id);
	return change_action (wallet, source_a, representative_a, work_a, generate_work_a);
}

nano::block_hash nano::wallets::send_sync (nano::wallet_id const & wallet_id, nano::account const & source_a, nano::account const & account_a, nano::uint128_t const & amount_a)
{
	nano::amount amount{ amount_a };
	nano::block_hash hash;
	rsnano::rsn_wallets_send_sync (rust_handle, wallet_id.bytes.data (), source_a.bytes.data (), account_a.bytes.data (), amount.bytes.data (), hash.bytes.data ());
	return hash;
}

bool nano::wallets::receive_sync (nano::wallet_id const & wallet_id, std::shared_ptr<nano::block> const & block_a, nano::account const & representative_a, nano::uint128_t const & amount_a)
{
	auto lock{ mutex.lock () };
	auto wallet = lock.find (wallet_id);
	return receive_sync (wallet, block_a, representative_a, amount_a);
}

bool nano::wallets::change_sync (nano::wallet_id const & wallet_id, nano::account const & source_a, nano::account const & representative_a)
{
	auto lock{ mutex.lock () };
	auto wallet = lock.find (wallet_id);
	return change_sync (wallet, source_a, representative_a);
}

nano::wallets_error nano::wallets::receive_async (nano::wallet_id const & wallet_id, nano::block_hash const & hash_a, nano::account const & representative_a, nano::uint128_t const & amount_a, nano::account const & account_a, std::function<void (std::shared_ptr<nano::block> const &)> const & action_a, uint64_t work_a, bool generate_work_a)
{
	nano::amount amount{ amount_a };
	auto context = new std::function<void (std::shared_ptr<nano::block> const &)> (action_a);
	auto result = rsnano::rsn_wallets_receive_async (rust_handle, wallet_id.bytes.data (), hash_a.bytes.data (), representative_a.bytes.data (), amount.bytes.data (), account_a.bytes.data (), start_election_wrapper, context, delete_start_election_context, work_a, generate_work_a);
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::change_async (nano::wallet_id const & wallet_id, nano::account const & source_a, nano::account const & representative_a, std::function<void (std::shared_ptr<nano::block> const &)> const & action_a, uint64_t work_a, bool generate_work_a)
{
	auto context = new std::function<void (std::shared_ptr<nano::block> const &)> (action_a);
	auto result = rsnano::rsn_wallets_change_async (rust_handle, wallet_id.bytes.data (), source_a.bytes.data (), representative_a.bytes.data (), start_election_wrapper, context, delete_start_election_context, work_a, generate_work_a);
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::send_async (nano::wallet_id const & wallet_id, nano::account const & source_a, nano::account const & account_a, nano::uint128_t const & amount_a, std::function<void (std::shared_ptr<nano::block> const &)> const & action_a, uint64_t work_a, bool generate_work_a, boost::optional<std::string> id_a)
{
	nano::amount amount{ amount_a };
	auto context = new std::function<void (std::shared_ptr<nano::block> const &)> (action_a);
	auto id_ptr = id_a.has_value () ? id_a.value ().c_str () : nullptr;
	auto result = rsnano::rsn_wallets_send_async (rust_handle, wallet_id.bytes.data (), source_a.bytes.data (), account_a.bytes.data (), amount.bytes.data (), start_election_wrapper, context, delete_start_election_context, work_a, generate_work_a, id_ptr);
	return static_cast<nano::wallets_error> (result);
}

nano::wallets_error nano::wallets::serialize (nano::wallet_id const & wallet_id, std::string & json)
{
	rsnano::StringDto json_dto;
	auto result = rsnano::rsn_wallets_serialize (rust_handle, wallet_id.bytes.data (), &json_dto);
	auto error = static_cast<nano::wallets_error> (result);
	if (error == nano::wallets_error::none)
	{
		json = rsnano::convert_dto_to_string (json_dto);
	}
	return error;
}

void nano::wallets::create (nano::wallet_id const & id_a)
{
	rsnano::rsn_wallets_create (rust_handle, id_a.bytes.data ());
}

nano::wallets_error nano::wallets::search_receivable (nano::wallet_id const & wallet_id)
{
	return static_cast<nano::wallets_error> (rsnano::rsn_wallets_search_receivable_wallet (rust_handle, wallet_id.bytes.data ()));
}

void nano::wallets::search_receivable_all ()
{
	rsnano::rsn_wallets_search_receivable_all (rust_handle);
}

void nano::wallets::destroy (nano::wallet_id const & id_a)
{
	rsnano::rsn_wallets_destroy (rust_handle, id_a.bytes.data ());
}

void nano::wallets::reload ()
{
	rsnano::rsn_wallets_reload (rust_handle);
}

namespace
{
void foreach_representative_action (void * context, const uint8_t * pub_key_bytes, const uint8_t * priv_key_bytes)
{
	auto action = static_cast<std::function<void (nano::public_key const & pub_a, nano::raw_key const & prv_a)> *> (context);
	auto pub_key{ nano::public_key::from_bytes (pub_key_bytes) };
	auto prv_key{ nano::raw_key::from_bytes (priv_key_bytes) };
	(*action) (pub_key, prv_key);
}

void delete_foreach_representative_context (void * context)
{
	auto action = static_cast<std::function<void (nano::public_key const & pub_a, nano::raw_key const & prv_a)> *> (context);
	delete action;
}
}

void nano::wallets::foreach_representative (std::function<void (nano::public_key const & pub_a, nano::raw_key const & prv_a)> const & action_a)
{
	auto context = new std::function<void (nano::public_key const & pub_a, nano::raw_key const & prv_a)> (action_a);
	rsnano::rsn_wallets_foreach_representative (
	rust_handle,
	foreach_representative_action,
	context,
	delete_foreach_representative_context);
}

bool nano::wallets::exists (nano::account const & account_a)
{
	return rsnano::rsn_wallets_exists (rust_handle, account_a.bytes.data ());
}

void nano::wallets::clear_send_ids ()
{
	rsnano::rsn_lmdb_wallets_clear_send_ids (rust_handle);
}

size_t nano::wallets::voting_reps_count () const
{
	return lock_representatives ().voting_reps ();
}

bool nano::wallets::have_half_rep () const
{
	return lock_representatives ().have_half_rep ();
}

bool nano::wallets::rep_exists (nano::account const & rep) const
{
	return lock_representatives ().exists (rep);
}

void nano::wallets::compute_reps ()
{
	rsnano::rsn_wallets_compute_reps (rust_handle);
}

namespace
{
nano::store::iterator<nano::account, nano::wallet_value> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return { nullptr };
	}

	return { std::make_unique<nano::store::lmdb::iterator<nano::account, nano::wallet_value>> (it_handle) };
}
}

nano::store::iterator<nano::account, nano::wallet_value> nano::wallet_store::begin (nano::store::transaction const & transaction_a)
{
	auto it_handle{ rsnano::rsn_lmdb_wallet_store_begin (rust_handle, transaction_a.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store::iterator<nano::account, nano::wallet_value> nano::wallet_store::find (store::transaction const & transaction_a, nano::account const & key)
{
	auto it_handle = rsnano::rsn_lmdb_wallet_store_find (rust_handle, transaction_a.get_rust_handle (), key.bytes.data ());
	return to_iterator (it_handle);
}

nano::store::iterator<nano::account, nano::wallet_value> nano::wallet_store::end ()
{
	return { nullptr };
}
nano::mdb_wallets_store::mdb_wallets_store (std::filesystem::path const & path_a, nano::lmdb_config const & lmdb_config_a) :
	environment (error, path_a, nano::store::lmdb::env::options::make ().set_config (lmdb_config_a).override_config_sync (nano::lmdb_config::sync_strategy::always).override_config_map_size (1ULL * 1024 * 1024 * 1024))
{
}

bool nano::mdb_wallets_store::init_error () const
{
	return error;
}
