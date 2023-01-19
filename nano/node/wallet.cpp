#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/election.hpp>
#include <nano/node/lmdb/lmdb_iterator.hpp>
#include <nano/node/node.hpp>
#include <nano/node/wallet.hpp>

#include <boost/filesystem.hpp>
#include <boost/format.hpp>
#include <boost/polymorphic_cast.hpp>
#include <boost/property_tree/json_parser.hpp>

#include <future>

nano::uint256_union nano::wallet_store::check (nano::transaction const & transaction_a)
{
	nano::uint256_union result;
	rsnano::rsn_lmdb_wallet_store_check (rust_handle, transaction_a.get_rust_handle (), result.bytes.data ());
	return result;
}

nano::uint256_union nano::wallet_store::salt (nano::transaction const & transaction_a)
{
	nano::uint256_union result;
	rsnano::rsn_lmdb_wallet_store_salt (rust_handle, transaction_a.get_rust_handle (), result.bytes.data ());
	return result;
}

void nano::wallet_store::wallet_key (nano::raw_key & prv_a, nano::transaction const & transaction_a)
{
	rsnano::rsn_lmdb_wallet_store_wallet_key (rust_handle, prv_a.bytes.data (), transaction_a.get_rust_handle ());
}

void nano::wallet_store::seed (nano::raw_key & prv_a, nano::transaction const & transaction_a)
{
	rsnano::rsn_lmdb_wallet_store_seed (rust_handle, prv_a.bytes.data (), transaction_a.get_rust_handle ());
}

void nano::wallet_store::seed_set (nano::transaction const & transaction_a, nano::raw_key const & prv_a)
{
	rsnano::rsn_lmdb_wallet_store_seed_set (rust_handle, transaction_a.get_rust_handle (), prv_a.bytes.data ());
}

nano::public_key nano::wallet_store::deterministic_insert (nano::transaction const & transaction_a)
{
	nano::public_key key;
	rsnano::rsn_lmdb_wallet_store_deterministic_insert (rust_handle, transaction_a.get_rust_handle (), key.bytes.data ());
	return key;
}

nano::public_key nano::wallet_store::deterministic_insert (nano::transaction const & transaction_a, uint32_t const index)
{
	nano::public_key key;
	rsnano::rsn_lmdb_wallet_store_deterministic_insert_at (rust_handle, transaction_a.get_rust_handle (), index, key.bytes.data ());
	return key;
}

nano::raw_key nano::wallet_store::deterministic_key (nano::transaction const & transaction_a, uint32_t index_a)
{
	nano::raw_key key;
	rsnano::rsn_lmdb_wallet_store_deterministic_key (rust_handle, transaction_a.get_rust_handle (), index_a, key.bytes.data ());
	return key;
}

uint32_t nano::wallet_store::deterministic_index_get (nano::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_wallet_store_deterministic_index_get (rust_handle, transaction_a.get_rust_handle ());
}

void nano::wallet_store::deterministic_index_set (nano::transaction const & transaction_a, uint32_t index_a)
{
	rsnano::rsn_lmdb_wallet_store_deterministic_index_set (rust_handle, transaction_a.get_rust_handle (), index_a);
}

void nano::wallet_store::deterministic_clear (nano::transaction const & transaction_a)
{
	rsnano::rsn_lmdb_wallet_store_deterministic_clear (rust_handle, transaction_a.get_rust_handle ());
}

bool nano::wallet_store::valid_password (nano::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_wallet_store_valid_password (rust_handle, transaction_a.get_rust_handle ());
}

bool nano::wallet_store::attempt_password (nano::transaction const & transaction_a, std::string const & password_a)
{
	return !rsnano::rsn_lmdb_wallet_store_attempt_password (rust_handle, transaction_a.get_rust_handle (), password_a.c_str ());
}

bool nano::wallet_store::rekey (nano::transaction const & transaction_a, std::string const & password_a)
{
	return !rsnano::rsn_lmdb_wallet_store_rekey (rust_handle, transaction_a.get_rust_handle (), password_a.c_str ());
}

void nano::wallet_store::derive_key (nano::raw_key & prv_a, nano::transaction const & transaction_a, std::string const & password_a)
{
	rsnano::rsn_lmdb_wallet_store_derive_key (rust_handle, prv_a.bytes.data (), transaction_a.get_rust_handle (), password_a.c_str ());
}

int const nano::wallet_store::special_count (7);

nano::wallet_store::wallet_store (bool & init_a, nano::kdf & kdf_a, nano::transaction & transaction_a, nano::account representative_a, unsigned fanout_a, std::string const & wallet_a, std::string const & json_a) :
	kdf (kdf_a),
	rust_handle{ rsnano::rsn_lmdb_wallet_store_create2 (fanout_a, kdf_a.handle, transaction_a.get_rust_handle (), wallet_a.c_str (), json_a.c_str ()) },
	fanout{ fanout_a }
{
	init_a = rust_handle == nullptr;
}

nano::wallet_store::wallet_store (bool & init_a, nano::kdf & kdf_a, nano::transaction & transaction_a, nano::account representative_a, unsigned fanout_a, std::string const & wallet_a) :
	kdf (kdf_a),
	rust_handle{ rsnano::rsn_lmdb_wallet_store_create (fanout_a, kdf_a.handle, transaction_a.get_rust_handle (), representative_a.bytes.data (), wallet_a.c_str ()) },
	fanout{ fanout_a }
{
	init_a = rust_handle == nullptr;
}

nano::wallet_store::~wallet_store ()
{
	if (rust_handle != nullptr)
		rsnano::rsn_lmdb_wallet_store_destroy (rust_handle);
}

bool nano::wallet_store::is_open () const
{
	return rsnano::rsn_lmdb_wallet_store_is_open (rust_handle);
}

void nano::wallet_store::lock ()
{
	rsnano::rsn_lmdb_wallet_store_lock (rust_handle);
}

void nano::wallet_store::password (nano::raw_key & password_a) const
{
	rsnano::rsn_lmdb_wallet_store_password (rust_handle, password_a.bytes.data ());
}

void nano::wallet_store::set_password (nano::raw_key const & password_a)
{
	rsnano::rsn_lmdb_wallet_store_set_password (rust_handle, password_a.bytes.data ());
}

std::vector<nano::account> nano::wallet_store::accounts (nano::transaction const & transaction_a)
{
	rsnano::U256ArrayDto dto;
	rsnano::rsn_lmdb_wallet_store_accounts (rust_handle, transaction_a.get_rust_handle (), &dto);
	std::vector<nano::account> result;
	result.reserve (dto.count);
	for (int i = 0; i < dto.count; ++i)
	{
		nano::account account;
		std::copy (std::begin (dto.items[i]), std::end (dto.items[i]), std::begin (account.bytes));
		result.push_back (account);
	}
	rsnano::rsn_u256_array_destroy (&dto);
	return result;
}

bool nano::wallet_store::is_representative (nano::transaction const & transaction_a)
{
	return exists (transaction_a, representative (transaction_a));
}

void nano::wallet_store::representative_set (nano::transaction const & transaction_a, nano::account const & representative_a)
{
	rsnano::rsn_lmdb_wallet_store_representative_set (rust_handle, transaction_a.get_rust_handle (), representative_a.bytes.data ());
}

nano::account nano::wallet_store::representative (nano::transaction const & transaction_a)
{
	nano::account rep;
	rsnano::rsn_lmdb_wallet_store_representative (rust_handle, transaction_a.get_rust_handle (), rep.bytes.data ());
	return rep;
}

nano::public_key nano::wallet_store::insert_adhoc (nano::transaction const & transaction_a, nano::raw_key const & prv)
{
	nano::public_key pub;
	rsnano::rsn_lmdb_wallet_store_insert_adhoc (rust_handle, transaction_a.get_rust_handle (), prv.bytes.data (), pub.bytes.data ());
	return pub;
}

bool nano::wallet_store::insert_watch (nano::transaction const & transaction_a, nano::account const & pub_a)
{
	return !rsnano::rsn_lmdb_wallet_store_insert_watch (rust_handle, transaction_a.get_rust_handle (), pub_a.bytes.data ());
}

void nano::wallet_store::erase (nano::transaction const & transaction_a, nano::account const & pub)
{
	rsnano::rsn_lmdb_wallet_store_erase (rust_handle, transaction_a.get_rust_handle (), pub.bytes.data ());
}

nano::key_type nano::wallet_store::key_type (nano::wallet_value const & value_a)
{
	auto dto{ value_a.to_dto () };
	return static_cast<nano::key_type> (rsnano::rsn_lmdb_wallet_store_key_type (&dto));
}

bool nano::wallet_store::fetch (nano::transaction const & transaction_a, nano::account const & pub, nano::raw_key & prv)
{
	return !rsnano::rsn_lmdb_wallet_store_fetch (rust_handle, transaction_a.get_rust_handle (), pub.bytes.data (), prv.bytes.data ());
}

bool nano::wallet_store::exists (nano::transaction const & transaction_a, nano::public_key const & pub)
{
	return rsnano::rsn_lmdb_wallet_store_exists (rust_handle, transaction_a.get_rust_handle (), pub.bytes.data ());
}

void nano::wallet_store::serialize_json (nano::transaction const & transaction_a, std::string & string_a)
{
	rsnano::StringDto dto;
	rsnano::rsn_lmdb_wallet_store_serialize_json (rust_handle, transaction_a.get_rust_handle (), &dto);
	string_a = rsnano::convert_dto_to_string (dto);
}

void nano::wallet_store::write_backup (nano::transaction const & transaction_a, boost::filesystem::path const & path_a)
{
	rsnano::rsn_lmdb_wallet_store_write_backup (rust_handle, transaction_a.get_rust_handle (), path_a.c_str ());
}

bool nano::wallet_store::move (nano::transaction const & transaction_a, nano::wallet_store & other_a, std::vector<nano::public_key> const & keys)
{
	return !rsnano::rsn_lmdb_wallet_store_move (rust_handle, transaction_a.get_rust_handle (), other_a.rust_handle, reinterpret_cast<const uint8_t *> (keys.data ()), keys.size ());
}

bool nano::wallet_store::import (nano::transaction const & transaction_a, nano::wallet_store & other_a)
{
	return !rsnano::rsn_lmdb_wallet_store_import (rust_handle, transaction_a.get_rust_handle (), other_a.rust_handle);
}

bool nano::wallet_store::work_get (nano::transaction const & transaction_a, nano::public_key const & pub_a, uint64_t & work_a)
{
	return !rsnano::rsn_lmdb_wallet_store_work_get (rust_handle, transaction_a.get_rust_handle (), pub_a.bytes.data (), &work_a);
}

void nano::wallet_store::work_put (nano::transaction const & transaction_a, nano::public_key const & pub_a, uint64_t work_a)
{
	rsnano::rsn_lmdb_wallet_store_work_put (rust_handle, transaction_a.get_rust_handle (), pub_a.bytes.data (), work_a);
}

unsigned nano::wallet_store::version (nano::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_wallet_store_version (rust_handle, transaction_a.get_rust_handle ());
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

nano::wallet::wallet (bool & init_a, nano::transaction & transaction_a, nano::wallets & wallets_a, std::string const & wallet_a) :
	store (init_a, wallets_a.kdf, transaction_a, wallets_a.node.config->random_representative (), wallets_a.node.config->password_fanout, wallet_a),
	wallets (wallets_a)
{
}

nano::wallet::wallet (bool & init_a, nano::transaction & transaction_a, nano::wallets & wallets_a, std::string const & wallet_a, std::string const & json) :
	store (init_a, wallets_a.kdf, transaction_a, wallets_a.node.config->random_representative (), wallets_a.node.config->password_fanout, wallet_a, json),
	wallets (wallets_a)
{
}

void nano::wallet::enter_initial_password ()
{
	nano::raw_key password_l;
	{
		nano::lock_guard<std::recursive_mutex> lock{ store.mutex };
		store.password (password_l);
	}
	if (password_l.is_zero ())
	{
		auto transaction (wallets.tx_begin_write ());
		if (store.valid_password (*transaction))
		{
			// Newly created wallets have a zero key
			store.rekey (*transaction, "");
		}
		else
		{
			enter_password (*transaction, "");
		}
	}
}

bool nano::wallet::enter_password (nano::transaction const & transaction_a, std::string const & password_a)
{
	auto result (store.attempt_password (transaction_a, password_a));
	if (!result)
	{
		auto this_l (shared_from_this ());
		wallets.node.background ([this_l] () {
			auto tx{ this_l->wallets.tx_begin_read () };
			this_l->search_receivable (*tx);
		});
		wallets.node.logger->try_log ("Wallet unlocked");
	}
	else
	{
		wallets.node.logger->try_log ("Invalid password, wallet locked");
	}
	return result;
}

nano::public_key nano::wallet::deterministic_insert (nano::transaction const & transaction_a, bool generate_work_a)
{
	nano::public_key key{};
	if (store.valid_password (transaction_a))
	{
		key = store.deterministic_insert (transaction_a);
		if (generate_work_a)
		{
			work_ensure (key, key);
		}
		auto half_principal_weight (wallets.node.minimum_principal_weight () / 2);
		if (wallets.check_rep (key, half_principal_weight))
		{
			nano::lock_guard<nano::mutex> lock{ representatives_mutex };
			representatives.insert (key);
		}
	}
	return key;
}

nano::public_key nano::wallet::deterministic_insert (uint32_t const index, bool generate_work_a)
{
	auto transaction (wallets.tx_begin_write ());
	nano::public_key key{};
	if (store.valid_password (*transaction))
	{
		key = store.deterministic_insert (*transaction, index);
		if (generate_work_a)
		{
			work_ensure (key, key);
		}
	}
	return key;
}

nano::public_key nano::wallet::deterministic_insert (bool generate_work_a)
{
	auto transaction (wallets.tx_begin_write ());
	auto result (deterministic_insert (*transaction, generate_work_a));
	return result;
}

nano::public_key nano::wallet::insert_adhoc (nano::raw_key const & key_a, bool generate_work_a)
{
	nano::public_key key{};
	auto transaction (wallets.tx_begin_write ());
	if (store.valid_password (*transaction))
	{
		key = store.insert_adhoc (*transaction, key_a);
		auto block_transaction (wallets.node.store.tx_begin_read ());
		if (generate_work_a)
		{
			work_ensure (key, wallets.node.ledger.latest_root (*block_transaction, key));
		}
		auto half_principal_weight (wallets.node.minimum_principal_weight () / 2);
		// Makes sure that the representatives container will
		// be in sync with any added keys.
		transaction->commit ();
		if (wallets.check_rep (key, half_principal_weight))
		{
			nano::lock_guard<nano::mutex> lock{ representatives_mutex };
			representatives.insert (key);
		}
	}
	return key;
}

bool nano::wallet::insert_watch (nano::transaction const & transaction_a, nano::public_key const & pub_a)
{
	return store.insert_watch (transaction_a, pub_a);
}

bool nano::wallet::exists (nano::public_key const & account_a)
{
	auto transaction (wallets.tx_begin_read ());
	return store.exists (*transaction, account_a);
}

bool nano::wallet::import (std::string const & json_a, std::string const & password_a)
{
	auto error (false);
	std::unique_ptr<nano::wallet_store> temp;
	{
		auto transaction (wallets.tx_begin_write ());
		nano::uint256_union id;
		random_pool::generate_block (id.bytes.data (), id.bytes.size ());
		temp = std::make_unique<nano::wallet_store> (error, wallets.node.wallets.kdf, *transaction, 0, 1, id.to_string (), json_a);
	}
	if (!error)
	{
		auto transaction (wallets.tx_begin_write ());
		error = temp->attempt_password (*transaction, password_a);
	}
	auto transaction (wallets.tx_begin_write ());
	if (!error)
	{
		error = store.import (*transaction, *temp);
	}
	temp->destroy (*transaction);
	return error;
}

void nano::wallet::serialize (std::string & json_a)
{
	auto transaction (wallets.tx_begin_read ());
	store.serialize_json (*transaction, json_a);
}

void nano::wallet_store::destroy (nano::transaction const & transaction_a)
{
	if (rust_handle != nullptr)
		rsnano::rsn_lmdb_wallet_store_destroy2 (rust_handle, transaction_a.get_rust_handle ());
}

std::shared_ptr<nano::block> nano::wallet::receive_action (nano::block_hash const & send_hash_a, nano::account const & representative_a, nano::uint128_union const & amount_a, nano::account const & account_a, uint64_t work_a, bool generate_work_a)
{
	std::shared_ptr<nano::block> block;
	nano::epoch epoch = nano::epoch::epoch_0;
	if (wallets.node.config->receive_minimum.number () <= amount_a.number ())
	{
		auto block_transaction (wallets.node.ledger.store.tx_begin_read ());
		auto transaction (wallets.tx_begin_read ());
		nano::pending_info pending_info;
		if (wallets.node.ledger.block_or_pruned_exists (*block_transaction, send_hash_a))
		{
			if (!wallets.node.ledger.store.pending ().get (*block_transaction, nano::pending_key (account_a, send_hash_a), pending_info))
			{
				nano::raw_key prv;
				if (!store.fetch (*transaction, account_a, prv))
				{
					if (work_a == 0)
					{
						store.work_get (*transaction, account_a, work_a);
					}
					nano::account_info info;
					auto new_account (wallets.node.ledger.store.account ().get (*block_transaction, account_a, info));
					if (!new_account)
					{
						block = std::make_shared<nano::state_block> (account_a, info.head (), info.representative (), info.balance ().number () + pending_info.amount.number (), send_hash_a, prv, account_a, work_a);
						epoch = std::max (info.epoch (), pending_info.epoch);
					}
					else
					{
						block = std::make_shared<nano::state_block> (account_a, 0, representative_a, pending_info.amount, reinterpret_cast<nano::link const &> (send_hash_a), prv, account_a, work_a);
						epoch = pending_info.epoch;
					}
				}
				else
				{
					wallets.node.logger->try_log ("Unable to receive, wallet locked");
				}
			}
			else
			{
				// Ledger doesn't have this marked as available to receive anymore
			}
		}
		else
		{
			// Ledger doesn't have this block anymore.
		}
	}
	else
	{
		wallets.node.logger->try_log (boost::str (boost::format ("Not receiving block %1% due to minimum receive threshold") % send_hash_a.to_string ()));
		// Someone sent us something below the threshold of receiving
	}
	if (block != nullptr)
	{
		auto details = nano::block_details (epoch, false, true, false);
		if (action_complete (block, account_a, generate_work_a, details))
		{
			// Return null block after work generation or ledger process error
			block = nullptr;
		}
	}
	return block;
}

std::shared_ptr<nano::block> nano::wallet::change_action (nano::account const & source_a, nano::account const & representative_a, uint64_t work_a, bool generate_work_a)
{
	auto epoch = nano::epoch::epoch_0;
	std::shared_ptr<nano::block> block;
	{
		auto transaction (wallets.tx_begin_read ());
		auto block_transaction (wallets.node.store.tx_begin_read ());
		if (store.valid_password (*transaction))
		{
			auto existing (store.find (*transaction, source_a));
			if (existing != store.end () && !wallets.node.ledger.latest (*block_transaction, source_a).is_zero ())
			{
				nano::account_info info;
				auto error1 (wallets.node.ledger.store.account ().get (*block_transaction, source_a, info));
				(void)error1;
				debug_assert (!error1);
				nano::raw_key prv;
				auto error2 (store.fetch (*transaction, source_a, prv));
				(void)error2;
				debug_assert (!error2);
				if (work_a == 0)
				{
					store.work_get (*transaction, source_a, work_a);
				}
				block = std::make_shared<nano::state_block> (source_a, info.head (), representative_a, info.balance (), 0, prv, source_a, work_a);
				epoch = info.epoch ();
			}
		}
	}
	if (block != nullptr)
	{
		auto details = nano::block_details (epoch, false, false, false);
		if (action_complete (block, source_a, generate_work_a, details))
		{
			// Return null block after work generation or ledger process error
			block = nullptr;
		}
	}
	return block;
}

std::shared_ptr<nano::block> nano::wallet::send_action (nano::account const & source_a, nano::account const & account_a, nano::uint128_t const & amount_a, uint64_t work_a, bool generate_work_a, boost::optional<std::string> id_a)
{
	auto prepare_send = [&id_a, &wallets = this->wallets, &store = this->store, &source_a, &amount_a, &work_a, &account_a] (auto const & transaction) {
		auto block_transaction (wallets.node.store.tx_begin_read ());

		auto error (false);
		std::shared_ptr<nano::block> block;
		if (id_a)
		{
			auto hash{ wallets.get_block_hash (error, *transaction, *id_a) };
			if (!hash.is_zero ())
			{
				block = wallets.node.store.block ().get (*block_transaction, hash);
			}
		}

		nano::block_details details = nano::block_details (nano::epoch::epoch_0, true, false, false);
		auto cached_block (false);
		if (block != nullptr)
		{
			cached_block = true;
			wallets.node.network->flood_block (block, nano::buffer_drop_policy::no_limiter_drop);
		}
		if (!error && block == nullptr)
		{
			if (store.valid_password (*transaction))
			{
				auto existing (store.find (*transaction, source_a));
				if (existing != store.end ())
				{
					auto balance (wallets.node.ledger.account_balance (*block_transaction, source_a));
					if (!balance.is_zero () && balance >= amount_a)
					{
						nano::account_info info;
						auto error1 (wallets.node.ledger.store.account ().get (*block_transaction, source_a, info));
						(void)error1;
						debug_assert (!error1);
						nano::raw_key prv;
						auto error2 (store.fetch (*transaction, source_a, prv));
						(void)error2;
						debug_assert (!error2);
						if (work_a == 0)
						{
							store.work_get (*transaction, source_a, work_a);
						}
						block = std::make_shared<nano::state_block> (source_a, info.head (), info.representative (), balance - amount_a, account_a, prv, source_a, work_a);
						details = nano::block_details (info.epoch (), details.is_send (), details.is_receive (), details.is_epoch ());
						if (id_a && block != nullptr)
						{
							error = wallets.set_block_hash (*transaction, *id_a, block->hash ());
							if (error)
							{
								block = nullptr;
							}
						}
					}
				}
			}
		}
		return std::make_tuple (block, error, cached_block, details);
	};

	std::tuple<std::shared_ptr<nano::block>, bool, bool, nano::block_details> result;
	{
		if (id_a)
		{
			result = prepare_send (wallets.tx_begin_write ());
		}
		else
		{
			result = prepare_send (wallets.tx_begin_read ());
		}
	}

	std::shared_ptr<nano::block> block;
	bool error;
	bool cached_block;
	nano::block_details details;
	std::tie (block, error, cached_block, details) = result;

	if (!error && block != nullptr && !cached_block)
	{
		if (action_complete (block, source_a, generate_work_a, details))
		{
			// Return null block after work generation or ledger process error
			block = nullptr;
		}
	}
	return block;
}

bool nano::wallet::action_complete (std::shared_ptr<nano::block> const & block_a, nano::account const & account_a, bool const generate_work_a, nano::block_details const & details_a)
{
	bool error{ false };
	// Unschedule any work caching for this account
	wallets.delayed_work->erase (account_a);
	if (block_a != nullptr)
	{
		auto required_difficulty{ wallets.node.network_params.work.threshold (block_a->work_version (), details_a) };
		if (wallets.node.network_params.work.difficulty (*block_a) < required_difficulty)
		{
			wallets.node.logger->try_log (boost::str (boost::format ("Cached or provided work for block %1% account %2% is invalid, regenerating") % block_a->hash ().to_string () % account_a.to_account ()));
			debug_assert (required_difficulty <= wallets.node.max_work_generate_difficulty (block_a->work_version ()));
			error = !wallets.node.work_generate_blocking (*block_a, required_difficulty).is_initialized ();
		}
		if (!error)
		{
			error = wallets.node.process_local (block_a).code != nano::process_result::progress;
			debug_assert (error || block_a->sideband ().details () == details_a);
		}
		if (!error && generate_work_a)
		{
			work_ensure (account_a, block_a->hash ());
		}
	}
	return error;
}

bool nano::wallet::change_sync (nano::account const & source_a, nano::account const & representative_a)
{
	std::promise<bool> result;
	std::future<bool> future = result.get_future ();
	change_async (
	source_a, representative_a, [&result] (std::shared_ptr<nano::block> const & block_a) {
		result.set_value (block_a == nullptr);
	},
	true);
	return future.get ();
}

void nano::wallet::change_async (nano::account const & source_a, nano::account const & representative_a, std::function<void (std::shared_ptr<nano::block> const &)> const & action_a, uint64_t work_a, bool generate_work_a)
{
	auto this_l (shared_from_this ());
	wallets.node.wallets.queue_wallet_action (nano::wallets::high_priority, this_l, [this_l, source_a, representative_a, action_a, work_a, generate_work_a] (nano::wallet & wallet_a) {
		auto block (wallet_a.change_action (source_a, representative_a, work_a, generate_work_a));
		action_a (block);
	});
}

bool nano::wallet::receive_sync (std::shared_ptr<nano::block> const & block_a, nano::account const & representative_a, nano::uint128_t const & amount_a)
{
	std::promise<bool> result;
	std::future<bool> future = result.get_future ();
	auto destination (block_a->link ().is_zero () ? block_a->destination () : block_a->link ().as_account ());
	receive_async (
	block_a->hash (), representative_a, amount_a, destination, [&result] (std::shared_ptr<nano::block> const & block_a) {
		result.set_value (block_a == nullptr);
	},
	true);
	return future.get ();
}

void nano::wallet::receive_async (nano::block_hash const & hash_a, nano::account const & representative_a, nano::uint128_t const & amount_a, nano::account const & account_a, std::function<void (std::shared_ptr<nano::block> const &)> const & action_a, uint64_t work_a, bool generate_work_a)
{
	auto this_l (shared_from_this ());
	wallets.node.wallets.queue_wallet_action (amount_a, this_l, [this_l, hash_a, representative_a, amount_a, account_a, action_a, work_a, generate_work_a] (nano::wallet & wallet_a) {
		auto block (wallet_a.receive_action (hash_a, representative_a, amount_a, account_a, work_a, generate_work_a));
		action_a (block);
	});
}

nano::block_hash nano::wallet::send_sync (nano::account const & source_a, nano::account const & account_a, nano::uint128_t const & amount_a)
{
	std::promise<nano::block_hash> result;
	std::future<nano::block_hash> future = result.get_future ();
	send_async (
	source_a, account_a, amount_a, [&result] (std::shared_ptr<nano::block> const & block_a) {
		result.set_value (block_a->hash ());
	},
	true);
	return future.get ();
}

void nano::wallet::send_async (nano::account const & source_a, nano::account const & account_a, nano::uint128_t const & amount_a, std::function<void (std::shared_ptr<nano::block> const &)> const & action_a, uint64_t work_a, bool generate_work_a, boost::optional<std::string> id_a)
{
	auto this_l (shared_from_this ());
	wallets.node.wallets.queue_wallet_action (nano::wallets::high_priority, this_l, [this_l, source_a, account_a, amount_a, action_a, work_a, generate_work_a, id_a] (nano::wallet & wallet_a) {
		auto block (wallet_a.send_action (source_a, account_a, amount_a, work_a, generate_work_a, id_a));
		action_a (block);
	});
}

// Update work for account if latest root is root_a
void nano::wallet::work_update (nano::transaction const & transaction_a, nano::account const & account_a, nano::root const & root_a, uint64_t work_a)
{
	debug_assert (!wallets.node.network_params.work.validate_entry (nano::work_version::work_1, root_a, work_a));
	debug_assert (store.exists (transaction_a, account_a));
	auto block_transaction (wallets.node.store.tx_begin_read ());
	auto latest (wallets.node.ledger.latest_root (*block_transaction, account_a));
	if (latest == root_a)
	{
		store.work_put (transaction_a, account_a, work_a);
	}
	else
	{
		wallets.node.logger->try_log ("Cached work no longer valid, discarding");
	}
}

void nano::wallet::work_ensure (nano::account const & account_a, nano::root const & root_a)
{
	using namespace std::chrono_literals;
	std::chrono::seconds const precache_delay = wallets.node.network_params.network.is_dev_network () ? 1s : 10s;

	wallets.delayed_work->operator[] (account_a) = root_a;

	wallets.node.workers->add_timed_task (std::chrono::steady_clock::now () + precache_delay, [this_l = shared_from_this (), account_a, root_a] {
		auto delayed_work = this_l->wallets.delayed_work.lock ();
		auto existing (delayed_work->find (account_a));
		if (existing != delayed_work->end () && existing->second == root_a)
		{
			delayed_work->erase (existing);
			this_l->wallets.queue_wallet_action (nano::wallets::generate_priority, this_l, [account_a, root_a] (nano::wallet & wallet_a) {
				wallet_a.work_cache_blocking (account_a, root_a);
			});
		}
	});
}

bool nano::wallet::search_receivable (nano::transaction const & wallet_transaction_a)
{
	auto result (!store.valid_password (wallet_transaction_a));
	if (!result)
	{
		wallets.node.logger->try_log ("Beginning receivable block search");
		for (auto i (store.begin (wallet_transaction_a)), n (store.end ()); i != n; ++i)
		{
			auto block_transaction (wallets.node.store.tx_begin_read ());
			nano::account const & account (i->first);
			// Don't search pending for watch-only accounts
			if (!nano::wallet_value (i->second).key.is_zero ())
			{
				for (auto j (wallets.node.store.pending ().begin (*block_transaction, nano::pending_key (account, 0))), k (wallets.node.store.pending ().end ()); j != k && nano::pending_key (j->first).account == account; ++j)
				{
					nano::pending_key key (j->first);
					auto hash (key.hash);
					nano::pending_info pending (j->second);
					auto amount (pending.amount.number ());
					if (wallets.node.config->receive_minimum.number () <= amount)
					{
						wallets.node.logger->try_log (boost::str (boost::format ("Found a receivable block %1% for account %2%") % hash.to_string () % pending.source.to_account ()));
						if (wallets.node.ledger.block_confirmed (*block_transaction, hash))
						{
							auto representative = store.representative (wallet_transaction_a);
							// Receive confirmed block
							receive_async (hash, representative, amount, account, [] (std::shared_ptr<nano::block> const &) {});
						}
						else if (!wallets.node.confirmation_height_processor.is_processing_block (hash))
						{
							auto block (wallets.node.store.block ().get (*block_transaction, hash));
							if (block)
							{
								// Request confirmation for block which is not being processed yet
								wallets.node.block_confirm (block);
							}
						}
					}
				}
			}
		}
		wallets.node.logger->try_log ("Receivable block search phase completed");
	}
	else
	{
		wallets.node.logger->try_log ("Stopping search, wallet is locked");
	}
	return result;
}

uint32_t nano::wallet::deterministic_check (nano::transaction const & transaction_a, uint32_t index)
{
	auto block_transaction (wallets.node.store.tx_begin_read ());
	for (uint32_t i (index + 1), n (index + 64); i < n; ++i)
	{
		auto prv = store.deterministic_key (transaction_a, i);
		nano::keypair pair (prv.to_string ());
		// Check if account received at least 1 block
		auto latest (wallets.node.ledger.latest (*block_transaction, pair.pub));
		if (!latest.is_zero ())
		{
			index = i;
			// i + 64 - Check additional 64 accounts
			// i/64 - Check additional accounts for large wallets. I.e. 64000/64 = 1000 accounts to check
			n = i + 64 + (i / 64);
		}
		else
		{
			// Check if there are pending blocks for account
			for (auto ii (wallets.node.store.pending ().begin (*block_transaction, nano::pending_key (pair.pub, 0))), nn (wallets.node.store.pending ().end ()); ii != nn && nano::pending_key (ii->first).account == pair.pub; ++ii)
			{
				index = i;
				n = i + 64 + (i / 64);
				break;
			}
		}
	}
	return index;
}

nano::public_key nano::wallet::change_seed (nano::transaction const & transaction_a, nano::raw_key const & prv_a, uint32_t count)
{
	store.seed_set (transaction_a, prv_a);
	auto account = deterministic_insert (transaction_a);
	if (count == 0)
	{
		count = deterministic_check (transaction_a, 0);
	}
	for (uint32_t i (0); i < count; ++i)
	{
		// Disable work generation to prevent weak CPU nodes stuck
		account = deterministic_insert (transaction_a, false);
	}
	return account;
}

void nano::wallet::deterministic_restore (nano::transaction const & transaction_a)
{
	auto index (store.deterministic_index_get (transaction_a));
	auto new_index (deterministic_check (transaction_a, index));
	for (uint32_t i (index); i <= new_index && index != new_index; ++i)
	{
		// Disable work generation to prevent weak CPU nodes stuck
		deterministic_insert (transaction_a, false);
	}
}

bool nano::wallet::live ()
{
	return store.is_open ();
}

void nano::wallet::work_cache_blocking (nano::account const & account_a, nano::root const & root_a)
{
	if (wallets.node.work_generation_enabled ())
	{
		auto difficulty (wallets.node.default_difficulty (nano::work_version::work_1));
		auto opt_work_l (wallets.node.work_generate_blocking (nano::work_version::work_1, root_a, difficulty, account_a));
		if (opt_work_l.is_initialized ())
		{
			auto transaction_l (wallets.tx_begin_write ());
			if (live () && store.exists (*transaction_l, account_a))
			{
				work_update (*transaction_l, account_a, root_a, *opt_work_l);
			}
		}
		else if (!wallets.node.stopped)
		{
			wallets.node.logger->try_log (boost::str (boost::format ("Could not precache work for root %1% due to work generation failure") % root_a.to_string ()));
		}
	}
}

void nano::wallets::do_wallet_actions ()
{
	nano::unique_lock<nano::mutex> action_lock{ action_mutex };
	while (!stopped)
	{
		if (!actions.empty ())
		{
			auto first (actions.begin ());
			auto wallet (first->second.first);
			auto current (std::move (first->second.second));
			actions.erase (first);
			if (wallet->live ())
			{
				action_lock.unlock ();
				observer (true);
				current (*wallet);
				observer (false);
				action_lock.lock ();
			}
		}
		else
		{
			condition.wait (action_lock);
		}
	}
}

nano::wallets::wallets (bool error_a, nano::node & node_a) :
	network_params{ node_a.config->network_params },
	observer ([] (bool) {}),
	kdf{ node_a.config->network_params.kdf_work },
	node (node_a),
	env (boost::polymorphic_downcast<nano::mdb_wallets_store *> (node_a.wallets_store_impl.get ())->environment),
	stopped (false),
	rust_handle{ rsnano::rsn_lmdb_wallets_create () }
{
	nano::unique_lock<nano::mutex> lock{ mutex };
	if (!error_a)
	{
		auto transaction (tx_begin_write ());
		auto store_l = dynamic_cast<nano::lmdb::store *> (&node.store);
		int status = !rsnano::rsn_lmdb_wallets_init (rust_handle, transaction->get_rust_handle (), store_l->handle);

		auto wallet_ids{ get_wallet_ids (*transaction) };
		for (auto id : wallet_ids)
		{
			release_assert (items.find (id) == items.end ());
			std::string text;
			id.encode_hex (text);
			bool error = false;
			auto wallet (std::make_shared<nano::wallet> (error, *transaction, *this, text));
			if (!error)
			{
				items[id] = wallet;
			}
			else
			{
				// Couldn't open wallet
			}
		}
	}
	// Backup before upgrade wallets
	bool backup_required (false);
	if (node.config->backup_before_upgrade)
	{
		auto transaction (tx_begin_read ());
		for (auto & item : items)
		{
			if (item.second->store.version (*transaction) != nano::wallet_store::version_current)
			{
				backup_required = true;
				break;
			}
		}
	}
	if (backup_required)
	{
		rsnano::rsn_lmdb_store_create_backup_file (env.handle, nano::to_logger_handle (node_a.logger));
	}
	for (auto & item : items)
	{
		item.second->enter_initial_password ();
	}
	if (node_a.config->enable_voting)
	{
		lock.unlock ();
		ongoing_compute_reps ();
	}
}

nano::wallets::~wallets ()
{
	stop ();
	rsnano::rsn_lmdb_wallets_destroy (rust_handle);
}

std::shared_ptr<nano::wallet> nano::wallets::open (nano::wallet_id const & id_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	std::shared_ptr<nano::wallet> result;
	auto existing (items.find (id_a));
	if (existing != items.end ())
	{
		result = existing->second;
	}
	return result;
}

std::shared_ptr<nano::wallet> nano::wallets::create (nano::wallet_id const & id_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	debug_assert (items.find (id_a) == items.end ());
	std::shared_ptr<nano::wallet> result;
	bool error;
	{
		auto transaction (tx_begin_write ());
		result = std::make_shared<nano::wallet> (error, *transaction, *this, id_a.to_string ());
	}
	if (!error)
	{
		items[id_a] = result;
		result->enter_initial_password ();
	}
	return result;
}

bool nano::wallets::search_receivable (nano::wallet_id const & wallet_a)
{
	auto result (false);
	if (auto wallet = open (wallet_a); wallet != nullptr)
	{
		auto tx{ tx_begin_read () };
		result = wallet->search_receivable (*tx);
	}
	return result;
}

void nano::wallets::search_receivable_all ()
{
	nano::unique_lock<nano::mutex> lk{ mutex };
	auto wallets_l = get_wallets ();
	auto wallet_transaction (tx_begin_read ());
	lk.unlock ();
	for (auto const & [id, wallet] : wallets_l)
	{
		wallet->search_receivable (*wallet_transaction);
	}
}

void nano::wallets::destroy (nano::wallet_id const & id_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	auto transaction (tx_begin_write ());
	// action_mutex should be after transactions to prevent deadlocks in deterministic_insert () & insert_adhoc ()
	nano::lock_guard<nano::mutex> action_lock{ action_mutex };
	auto existing (items.find (id_a));
	debug_assert (existing != items.end ());
	auto wallet (existing->second);
	items.erase (existing);
	wallet->store.destroy (*transaction);
}

void nano::wallets::reload ()
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	auto transaction (tx_begin_write ());
	std::unordered_set<nano::uint256_union> stored_items;
	auto wallet_ids{ get_wallet_ids (*transaction) };
	for (auto id : wallet_ids)
	{
		// New wallet
		if (items.find (id) == items.end ())
		{
			bool error = false;
			std::string text;
			id.encode_hex (text);
			auto wallet (std::make_shared<nano::wallet> (error, *transaction, *this, text));
			if (!error)
			{
				items[id] = wallet;
			}
		}
		// List of wallets on disk
		stored_items.insert (id);
	}
	// Delete non existing wallets from memory
	std::vector<nano::wallet_id> deleted_items;
	for (auto i : items)
	{
		if (stored_items.find (i.first) == stored_items.end ())
		{
			deleted_items.push_back (i.first);
		}
	}
	for (auto & i : deleted_items)
	{
		debug_assert (items.find (i) == items.end ());
		items.erase (i);
	}
}

void nano::wallets::queue_wallet_action (nano::uint128_t const & amount_a, std::shared_ptr<nano::wallet> const & wallet_a, std::function<void (nano::wallet &)> action_a)
{
	{
		nano::lock_guard<nano::mutex> action_lock{ action_mutex };
		actions.emplace (amount_a, std::make_pair (wallet_a, action_a));
	}
	condition.notify_all ();
}

void nano::wallets::foreach_representative (std::function<void (nano::public_key const & pub_a, nano::raw_key const & prv_a)> const & action_a)
{
	if (node.config->enable_voting)
	{
		std::vector<std::pair<nano::public_key const, nano::raw_key const>> action_accounts_l;
		{
			auto transaction_l (tx_begin_read ());
			nano::lock_guard<nano::mutex> lock{ mutex };
			for (auto i (items.begin ()), n (items.end ()); i != n; ++i)
			{
				auto & wallet (*i->second);
				nano::lock_guard<std::recursive_mutex> store_lock{ wallet.store.mutex };
				decltype (wallet.representatives) representatives_l;
				{
					nano::lock_guard<nano::mutex> representatives_lock{ wallet.representatives_mutex };
					representatives_l = wallet.representatives;
				}
				for (auto const & account : representatives_l)
				{
					if (wallet.store.exists (*transaction_l, account))
					{
						if (!node.ledger.weight (account).is_zero ())
						{
							if (wallet.store.valid_password (*transaction_l))
							{
								nano::raw_key prv;
								auto error (wallet.store.fetch (*transaction_l, account, prv));
								(void)error;
								debug_assert (!error);
								action_accounts_l.emplace_back (account, prv);
							}
							else
							{
								static auto last_log = std::chrono::steady_clock::time_point ();
								if (last_log < std::chrono::steady_clock::now () - std::chrono::seconds (60))
								{
									last_log = std::chrono::steady_clock::now ();
									node.logger->always_log (boost::str (boost::format ("Representative locked inside wallet %1%") % i->first.to_string ()));
								}
							}
						}
					}
				}
			}
		}
		for (auto const & representative : action_accounts_l)
		{
			action_a (representative.first, representative.second);
		}
	}
}

bool nano::wallets::exists (nano::transaction const & transaction_a, nano::account const & account_a)
{
	nano::lock_guard<nano::mutex> lock{ mutex };
	auto result (false);
	for (auto i (items.begin ()), n (items.end ()); !result && i != n; ++i)
	{
		result = i->second->store.exists (transaction_a, account_a);
	}
	return result;
}

void nano::wallets::stop ()
{
	{
		nano::lock_guard<nano::mutex> action_lock{ action_mutex };
		stopped = true;
		actions.clear ();
	}
	condition.notify_all ();
	if (thread.joinable ())
	{
		thread.join ();
	}
}

void nano::wallets::start ()
{
	thread = std::thread{ [this] () {
		nano::thread_role::set (nano::thread_role::name::wallet_actions);
		do_wallet_actions ();
	} };
}

std::unique_ptr<nano::write_transaction> nano::wallets::tx_begin_write ()
{
	return env.tx_begin_write ();
}

std::unique_ptr<nano::read_transaction> nano::wallets::tx_begin_read ()
{
	return env.tx_begin_read ();
}

void nano::wallets::clear_send_ids (nano::transaction const & transaction_a)
{
	rsnano::rsn_lmdb_wallets_clear_send_ids (rust_handle, transaction_a.get_rust_handle ());
}

nano::wallet_representatives nano::wallets::reps () const
{
	nano::lock_guard<nano::mutex> counts_guard{ reps_cache_mutex };
	return representatives;
}

bool nano::wallets::check_rep (nano::account const & account_a, nano::uint128_t const & half_principal_weight_a, bool const acquire_lock_a)
{
	auto weight = node.ledger.weight (account_a);

	if (weight < node.config->vote_minimum.number ())
	{
		return false; // account not a representative
	}

	nano::unique_lock<nano::mutex> lock;
	if (acquire_lock_a)
	{
		lock = nano::unique_lock<nano::mutex>{ reps_cache_mutex };
	}

	if (weight >= half_principal_weight_a)
	{
		representatives.half_principal = true;
	}

	auto insert_result = representatives.accounts.insert (account_a);
	if (!insert_result.second)
	{
		return false; // account already exists
	}

	++representatives.voting;

	return true;
}

void nano::wallets::compute_reps ()
{
	nano::lock_guard<nano::mutex> guard{ mutex };
	nano::lock_guard<nano::mutex> counts_guard{ reps_cache_mutex };
	representatives.clear ();
	auto half_principal_weight (node.minimum_principal_weight () / 2);
	auto transaction (tx_begin_read ());
	for (auto i (items.begin ()), n (items.end ()); i != n; ++i)
	{
		auto & wallet (*i->second);
		decltype (wallet.representatives) representatives_l;
		for (auto ii (wallet.store.begin (*transaction)), nn (wallet.store.end ()); ii != nn; ++ii)
		{
			auto account (ii->first);
			if (check_rep (account, half_principal_weight, false))
			{
				representatives_l.insert (account);
			}
		}
		nano::lock_guard<nano::mutex> representatives_guard{ wallet.representatives_mutex };
		wallet.representatives.swap (representatives_l);
	}
}

void nano::wallets::ongoing_compute_reps ()
{
	compute_reps ();
	auto & node_l (node);
	// Representation drifts quickly on the test network but very slowly on the live network
	auto compute_delay = network_params.network.is_dev_network () ? std::chrono::milliseconds (10) : (network_params.network.is_test_network () ? std::chrono::milliseconds (nano::test_scan_wallet_reps_delay ()) : std::chrono::minutes (15));
	node.workers->add_timed_task (std::chrono::steady_clock::now () + compute_delay, [&node_l] () {
		node_l.wallets.ongoing_compute_reps ();
	});
}

std::vector<nano::wallet_id> nano::wallets::get_wallet_ids (nano::transaction const & transaction_a)
{
	rsnano::U256ArrayDto dto;
	rsnano::rsn_lmdb_wallets_get_wallet_ids (rust_handle, transaction_a.get_rust_handle (), &dto);
	std::vector<nano::wallet_id> wallet_ids;
	for (int i = 0; i < dto.count; ++i)
	{
		nano::wallet_id id;
		std::copy (std::begin (dto.items[i]), std::end (dto.items[i]), std::begin (id.bytes));
		wallet_ids.push_back (id);
	}
	rsnano::rsn_u256_array_destroy (&dto);
	return wallet_ids;
}

nano::block_hash nano::wallets::get_block_hash (bool & error_a, nano::transaction const & transaction_a, std::string const & id_a)
{
	nano::block_hash result;
	error_a = !rsnano::rsn_lmdb_wallets_get_block_hash (rust_handle, transaction_a.get_rust_handle (), id_a.c_str (), result.bytes.data ());
	return result;
}

bool nano::wallets::set_block_hash (nano::transaction const & transaction_a, std::string const & id_a, nano::block_hash const & hash)
{
	return !rsnano::rsn_lmdb_wallets_set_block_hash (rust_handle, transaction_a.get_rust_handle (), id_a.c_str (), hash.bytes.data ());
}

std::unordered_map<nano::wallet_id, std::shared_ptr<nano::wallet>> nano::wallets::get_wallets ()
{
	debug_assert (!mutex.try_lock ());
	return items;
}

nano::uint128_t const nano::wallets::generate_priority = std::numeric_limits<nano::uint128_t>::max ();
nano::uint128_t const nano::wallets::high_priority = std::numeric_limits<nano::uint128_t>::max () - 1;

namespace
{
nano::store_iterator<nano::account, nano::wallet_value> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return { nullptr };
	}

	return { std::make_unique<nano::mdb_iterator<nano::account, nano::wallet_value>> (it_handle) };
}
}

nano::store_iterator<nano::account, nano::wallet_value> nano::wallet_store::begin (nano::transaction const & transaction_a)
{
	auto it_handle{ rsnano::rsn_lmdb_wallet_store_begin (rust_handle, transaction_a.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::wallet_value> nano::wallet_store::begin (nano::transaction const & transaction_a, nano::account const & key)
{
	auto it_handle{ rsnano::rsn_lmdb_wallet_store_begin_at_account (rust_handle, transaction_a.get_rust_handle (), key.bytes.data ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::wallet_value> nano::wallet_store::find (nano::transaction const & transaction_a, nano::account const & key)
{
	auto it_handle = rsnano::rsn_lmdb_wallet_store_find (rust_handle, transaction_a.get_rust_handle (), key.bytes.data ());
	return to_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::wallet_value> nano::wallet_store::end ()
{
	return { nullptr };
}
nano::mdb_wallets_store::mdb_wallets_store (boost::filesystem::path const & path_a, nano::lmdb_config const & lmdb_config_a) :
	environment (error, path_a, nano::mdb_env::options::make ().set_config (lmdb_config_a).override_config_sync (nano::lmdb_config::sync_strategy::always).override_config_map_size (1ULL * 1024 * 1024 * 1024))
{
}

bool nano::mdb_wallets_store::init_error () const
{
	return error;
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (wallets & wallets, std::string const & name)
{
	std::size_t items_count;
	std::size_t actions_count;
	{
		nano::lock_guard<nano::mutex> guard{ wallets.mutex };
		items_count = wallets.items.size ();
		actions_count = wallets.actions.size ();
	}

	auto sizeof_item_element = sizeof (decltype (wallets.items)::value_type);
	auto sizeof_actions_element = sizeof (decltype (wallets.actions)::value_type);
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "items", items_count, sizeof_item_element }));
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "actions", actions_count, sizeof_actions_element }));
	return composite;
}
