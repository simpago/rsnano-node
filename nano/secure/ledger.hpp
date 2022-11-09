#pragma once

#include <nano/lib/rep_weights.hpp>
#include <nano/lib/timer.hpp>
#include <nano/secure/common.hpp>

#include <map>

namespace nano
{
class store;
class stat;
class write_transaction;

// map of vote weight per block, ordered greater first
using tally_t = std::map<nano::uint128_t, std::shared_ptr<nano::block>, std::greater<nano::uint128_t>>;

class uncemented_info
{
public:
	uncemented_info (nano::block_hash const & cemented_frontier, nano::block_hash const & frontier, nano::account const & account);
	nano::block_hash cemented_frontier;
	nano::block_hash frontier;
	nano::account account;
};

class ledger final
{
public:
	ledger (nano::store &, nano::stat &, nano::ledger_constants & constants, nano::generate_cache const & = nano::generate_cache ());
	ledger (nano::ledger const &) = delete;
	ledger (nano::ledger &&) = delete;
	~ledger ();
	/**
	 * Return account containing hash, expects that block hash exists in ledger
	 */
	nano::account account (nano::transaction const &, nano::block_hash const &) const;
	/**
	 * For non-prunning nodes same as `ledger::account()`
	 * For prunning nodes ensures that block hash exists, otherwise returns zero account
	 */
	nano::account account_safe (nano::transaction const &, nano::block_hash const &, bool &) const;
	/**
	 * Return account containing hash, returns zero account if account can not be found
	 */
	nano::account account_safe (nano::transaction const &, nano::block_hash const &) const;
	nano::uint128_t amount (nano::transaction const &, nano::account const &);
	nano::uint128_t amount (nano::transaction const &, nano::block_hash const &);
	/** Safe for previous block, but block hash_a must exist */
	nano::uint128_t amount_safe (nano::transaction const &, nano::block_hash const & hash_a, bool &) const;
	nano::uint128_t balance (nano::transaction const &, nano::block_hash const &) const;
	nano::uint128_t balance_safe (nano::transaction const &, nano::block_hash const &, bool &) const;
	nano::uint128_t account_balance (nano::transaction const &, nano::account const &, bool = false);
	nano::uint128_t account_receivable (nano::transaction const &, nano::account const &, bool = false);
	nano::uint128_t weight (nano::account const &);
	std::shared_ptr<nano::block> successor (nano::transaction const &, nano::qualified_root const &);
	bool block_confirmed (nano::transaction const &, nano::block_hash const &) const;
	nano::block_hash latest (nano::transaction const &, nano::account const &);
	nano::root latest_root (nano::transaction const &, nano::account const &);
	nano::block_hash representative (nano::transaction const &, nano::block_hash const &);
	bool block_or_pruned_exists (nano::block_hash const &) const;
	bool block_or_pruned_exists (nano::transaction const &, nano::block_hash const &) const;
	std::string block_text (char const *);
	std::string block_text (nano::block_hash const &);
	bool is_send (nano::transaction const &, nano::block const &) const;
	nano::account block_destination (nano::transaction const &, nano::block const &);
	nano::block_hash block_source (nano::transaction const &, nano::block const &);
	std::pair<nano::block_hash, nano::block_hash> hash_root_random (nano::transaction const &) const;
	nano::process_return process (nano::write_transaction const &, nano::block &, nano::signature_verification = nano::signature_verification::unknown);
	bool rollback (nano::write_transaction const &, nano::block_hash const &, std::vector<std::shared_ptr<nano::block>> &);
	bool rollback (nano::write_transaction const &, nano::block_hash const &);
	void update_account (nano::write_transaction const &, nano::account const &, nano::account_info const &, nano::account_info const &);
	uint64_t pruning_action (nano::write_transaction &, nano::block_hash const &, uint64_t const);
	bool could_fit (nano::transaction const &, nano::block const &) const;
	bool dependents_confirmed (nano::transaction const &, nano::block const &) const;
	bool is_epoch_link (nano::link const &) const;
	std::array<nano::block_hash, 2> dependent_blocks (nano::transaction const &, nano::block const &) const;
	std::shared_ptr<nano::block> find_receive_block_by_send_hash (nano::transaction const & transaction, nano::account const & destination, nano::block_hash const & send_block_hash);
	nano::account epoch_signer (nano::link const &) const;
	nano::link epoch_link (nano::epoch) const;
	std::multimap<uint64_t, uncemented_info, std::greater<>> unconfirmed_frontiers () const;
	bool bootstrap_weight_reached () const;
	rsnano::LedgerHandle * get_handle () const;
	void write_confirmation_height (nano::write_transaction const &, nano::account const &, uint64_t num_blocks_cemented, uint64_t confirmation_height, nano::block_hash const & confirmed_frontier);
	size_t get_bootstrap_weights_size () const;
	void enable_pruning ();
	bool pruning_enabled () const;
	std::unordered_map<nano::account, nano::uint128_t> get_bootstrap_weights () const;
	void set_bootstrap_weights (std::unordered_map<nano::account, nano::uint128_t> const & weights_a);
	void set_bootstrap_weight_max_blocks (uint64_t max_a);
	uint64_t get_bootstrap_weight_max_blocks () const;
	static nano::uint128_t const unit;
	nano::store & store;
	nano::ledger_cache cache;
	nano::ledger_constants & constants;

private:
	nano::stat & stats;

private:
	rsnano::LedgerHandle * handle;
};

std::unique_ptr<container_info_component> collect_container_info (ledger & ledger, std::string const & name);
}
