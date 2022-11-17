#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/config.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/timer.hpp>
#include <nano/secure/common.hpp>
#include <nano/secure/store.hpp>

#include <boost/endian/conversion.hpp>
#include <boost/property_tree/json_parser.hpp>

#include <queue>

namespace
{
char const * dev_private_key_data = "34F0A37AAD20F4A260F0A5B3CB3D7FB50673212263E58A380BC10474BB039CE4";
}

nano::keypair nano::dev::genesis_key{ dev_private_key_data };
nano::network_params nano::dev::network_params{ nano::networks::nano_dev_network };
nano::ledger_constants & nano::dev::constants{ nano::dev::network_params.ledger };
std::shared_ptr<nano::block> & nano::dev::genesis = nano::dev::constants.genesis;

nano::network_params::network_params (nano::networks network_a) :
	work (nano::work_thresholds (0, 0, 0)),
	network (nano::network_constants (nano::work_thresholds (0, 0, 0), network_a)),
	ledger (nano::ledger_constants (nano::work_thresholds (0, 0, 0), network_a))
{
	rsnano::NetworkParamsDto dto;
	if (rsnano::rsn_network_params_create (&dto, static_cast<uint16_t> (network_a)) < 0)
		throw std::runtime_error ("could not create network params");

	work = nano::work_thresholds (dto.work);
	network = nano::network_constants (dto.network);
	ledger = std::move (nano::ledger_constants (dto.ledger));
	voting = nano::voting_constants (dto.voting);
	node = nano::node_constants (dto.node);
	portmapping = nano::portmapping_constants (dto.portmapping);
	bootstrap = nano::bootstrap_constants (dto.bootstrap);
	kdf_work = dto.kdf_work;
}

nano::network_params::network_params (rsnano::NetworkParamsDto const & dto) :
	kdf_work{ dto.kdf_work },
	work{ dto.work },
	network{ dto.network },
	ledger{ dto.ledger },
	voting{ dto.voting },
	node{ dto.node },
	portmapping{ dto.portmapping },
	bootstrap{ dto.bootstrap }
{
}

nano::NetworkParamsDtoWrapper nano::network_params::to_dto () const
{
	rsnano::NetworkParamsDto dto;
	dto.kdf_work = kdf_work;
	dto.work = work.dto;
	dto.network = network.to_dto ();
	dto.ledger = ledger.to_dto ();
	dto.voting = voting.to_dto ();
	dto.node = node.to_dto ();
	dto.portmapping = portmapping.to_dto ();
	dto.bootstrap = bootstrap.to_dto ();
	return NetworkParamsDtoWrapper{ dto };
}

nano::ledger_constants::ledger_constants (nano::work_thresholds work_a, nano::networks network_a) :
	work (nano::work_thresholds (0, 0, 0))
{
	rsnano::LedgerConstantsDto dto;
	if (rsnano::rsn_ledger_constants_create (&dto, &work_a.dto, static_cast<uint16_t> (network_a)) < 0)
		throw std::runtime_error ("could not create ledger_constants.");
	read_dto (dto);
}

nano::ledger_constants::ledger_constants (rsnano::LedgerConstantsDto const & dto) :
	work (nano::work_thresholds (0, 0, 0))
{
	read_dto (dto);
}

rsnano::LedgerConstantsDto nano::ledger_constants::to_dto () const
{
	rsnano::LedgerConstantsDto dto;
	dto.work = work.dto;
	std::copy (std::begin (zero_key.prv.bytes), std::end (zero_key.prv.bytes), std::begin (dto.priv_key));
	std::copy (std::begin (zero_key.pub.bytes), std::end (zero_key.pub.bytes), std::begin (dto.pub_key));
	std::copy (std::begin (nano_beta_account.bytes), std::end (nano_beta_account.bytes), std::begin (dto.nano_beta_account));
	std::copy (std::begin (nano_live_account.bytes), std::end (nano_live_account.bytes), std::begin (dto.nano_live_account));
	std::copy (std::begin (nano_test_account.bytes), std::end (nano_test_account.bytes), std::begin (dto.nano_test_account));

	dto.nano_dev_genesis = nano_dev_genesis->clone_handle ();
	dto.nano_beta_genesis = nano_beta_genesis->clone_handle ();
	dto.nano_live_genesis = nano_live_genesis->clone_handle ();
	dto.nano_test_genesis = nano_test_genesis->clone_handle ();
	dto.genesis = genesis->clone_handle ();
	boost::multiprecision::export_bits (genesis_amount, std::rbegin (dto.genesis_amount), 8, false);
	std::copy (std::begin (burn_account.bytes), std::end (burn_account.bytes), std::begin (dto.burn_account));
	std::copy (std::begin (nano_dev_final_votes_canary_account.bytes), std::end (nano_dev_final_votes_canary_account.bytes), std::begin (dto.nano_dev_final_votes_canary_account));
	std::copy (std::begin (nano_beta_final_votes_canary_account.bytes), std::end (nano_beta_final_votes_canary_account.bytes), std::begin (dto.nano_beta_final_votes_canary_account));
	std::copy (std::begin (nano_live_final_votes_canary_account.bytes), std::end (nano_live_final_votes_canary_account.bytes), std::begin (dto.nano_live_final_votes_canary_account));
	std::copy (std::begin (nano_test_final_votes_canary_account.bytes), std::end (nano_test_final_votes_canary_account.bytes), std::begin (dto.nano_test_final_votes_canary_account));
	std::copy (std::begin (final_votes_canary_account.bytes), std::end (final_votes_canary_account.bytes), std::begin (dto.final_votes_canary_account));
	dto.nano_dev_final_votes_canary_height = nano_dev_final_votes_canary_height;
	dto.nano_beta_final_votes_canary_height = nano_beta_final_votes_canary_height;
	dto.nano_live_final_votes_canary_height = nano_live_final_votes_canary_height;
	dto.nano_test_final_votes_canary_height = nano_test_final_votes_canary_height;
	dto.final_votes_canary_height = final_votes_canary_height;

	auto epoch_1_link{ epochs.link (nano::epoch::epoch_1) };
	auto epoch_1_signer{ epochs.signer (nano::epoch::epoch_1) };
	auto epoch_2_link{ epochs.link (nano::epoch::epoch_2) };
	auto epoch_2_signer{ epochs.signer (nano::epoch::epoch_2) };

	std::copy (std::begin (epoch_1_signer.bytes), std::end (epoch_1_signer.bytes), std::begin (dto.epoch_1_signer));
	std::copy (std::begin (epoch_1_link.bytes), std::end (epoch_1_link.bytes), std::begin (dto.epoch_1_link));
	std::copy (std::begin (epoch_2_signer.bytes), std::end (epoch_2_signer.bytes), std::begin (dto.epoch_2_signer));
	std::copy (std::begin (epoch_2_link.bytes), std::end (epoch_2_link.bytes), std::begin (dto.epoch_2_link));
	return dto;
}

void nano::ledger_constants::read_dto (rsnano::LedgerConstantsDto const & dto)
{
	work = nano::work_thresholds (dto.work);
	nano::public_key pub_key;
	nano::raw_key priv_key;
	std::copy (std::begin (dto.pub_key), std::end (dto.pub_key), std::begin (pub_key.bytes));
	std::copy (std::begin (dto.priv_key), std::end (dto.priv_key), std::begin (priv_key.bytes));
	zero_key = nano::keypair (priv_key, pub_key);
	std::copy (std::begin (dto.nano_beta_account), std::end (dto.nano_beta_account), std::begin (nano_beta_account.bytes));
	std::copy (std::begin (dto.nano_live_account), std::end (dto.nano_live_account), std::begin (nano_live_account.bytes));
	std::copy (std::begin (dto.nano_test_account), std::end (dto.nano_test_account), std::begin (nano_test_account.bytes));
	nano_dev_genesis = nano::block_handle_to_block (dto.nano_dev_genesis);
	nano_beta_genesis = nano::block_handle_to_block (dto.nano_beta_genesis);
	nano_live_genesis = nano::block_handle_to_block (dto.nano_live_genesis);
	nano_test_genesis = nano::block_handle_to_block (dto.nano_test_genesis);
	genesis = nano::block_handle_to_block (dto.genesis);
	boost::multiprecision::import_bits (genesis_amount, std::begin (dto.genesis_amount), std::end (dto.genesis_amount));
	std::copy (std::begin (dto.burn_account), std::end (dto.burn_account), std::begin (burn_account.bytes));
	std::copy (std::begin (dto.nano_dev_final_votes_canary_account), std::end (dto.nano_dev_final_votes_canary_account), std::begin (nano_dev_final_votes_canary_account.bytes));
	std::copy (std::begin (dto.nano_beta_final_votes_canary_account), std::end (dto.nano_beta_final_votes_canary_account), std::begin (nano_beta_final_votes_canary_account.bytes));
	std::copy (std::begin (dto.nano_live_final_votes_canary_account), std::end (dto.nano_live_final_votes_canary_account), std::begin (nano_live_final_votes_canary_account.bytes));
	std::copy (std::begin (dto.nano_test_final_votes_canary_account), std::end (dto.nano_test_final_votes_canary_account), std::begin (nano_test_final_votes_canary_account.bytes));
	std::copy (std::begin (dto.final_votes_canary_account), std::end (dto.final_votes_canary_account), std::begin (final_votes_canary_account.bytes));
	nano_dev_final_votes_canary_height = dto.nano_dev_final_votes_canary_height;
	nano_beta_final_votes_canary_height = dto.nano_beta_final_votes_canary_height;
	nano_live_final_votes_canary_height = dto.nano_live_final_votes_canary_height;
	nano_test_final_votes_canary_height = dto.nano_test_final_votes_canary_height;
	final_votes_canary_height = dto.final_votes_canary_height;

	nano::account epoch_v1_signer;
	std::copy (std::begin (dto.epoch_1_signer), std::end (dto.epoch_1_signer), std::begin (epoch_v1_signer.bytes));
	nano::link epoch_v1_link;
	std::copy (std::begin (dto.epoch_1_link), std::end (dto.epoch_1_link), std::begin (epoch_v1_link.bytes));
	nano::account epoch_v2_signer;
	std::copy (std::begin (dto.epoch_2_signer), std::end (dto.epoch_2_signer), std::begin (epoch_v2_signer.bytes));
	nano::link epoch_v2_link;
	std::copy (std::begin (dto.epoch_2_link), std::end (dto.epoch_2_link), std::begin (epoch_v2_link.bytes));

	epochs.add (nano::epoch::epoch_1, epoch_v1_signer, epoch_v1_link);
	epochs.add (nano::epoch::epoch_2, epoch_v2_signer, epoch_v2_link);
}

nano::hardened_constants & nano::hardened_constants::get ()
{
	static hardened_constants instance{};
	return instance;
}

nano::hardened_constants::hardened_constants () :
	not_an_account{},
	random_128{}
{
	rsnano::rsn_hardened_constants_get (not_an_account.bytes.data (), random_128.bytes.data ());
}

nano::node_constants::node_constants (rsnano::NodeConstantsDto const & dto)
{
	read_dto (dto);
}

void nano::node_constants::read_dto (rsnano::NodeConstantsDto const & dto)
{
	backup_interval = std::chrono::minutes (dto.backup_interval_m);
	search_pending_interval = std::chrono::seconds (dto.search_pending_interval_s);
	unchecked_cleaning_interval = std::chrono::minutes (dto.unchecked_cleaning_interval_m);
	process_confirmed_interval = std::chrono::milliseconds (dto.process_confirmed_interval_ms);
	max_weight_samples = dto.max_weight_samples;
	weight_period = dto.weight_period;
}

rsnano::NodeConstantsDto nano::node_constants::to_dto () const
{
	rsnano::NodeConstantsDto dto;
	dto.backup_interval_m = backup_interval.count ();
	dto.search_pending_interval_s = search_pending_interval.count ();
	dto.unchecked_cleaning_interval_m = unchecked_cleaning_interval.count ();
	dto.process_confirmed_interval_ms = process_confirmed_interval.count ();
	dto.max_weight_samples = max_weight_samples;
	dto.weight_period = weight_period;
	return dto;
}

nano::voting_constants::voting_constants (rsnano::VotingConstantsDto const & dto)
{
	max_cache = dto.max_cache;
	delay = std::chrono::seconds (dto.delay_s);
}

rsnano::VotingConstantsDto nano::voting_constants::to_dto () const
{
	rsnano::VotingConstantsDto result;
	result.max_cache = max_cache;
	result.delay_s = delay.count ();
	return result;
}

nano::portmapping_constants::portmapping_constants (nano::network_constants & network_constants)
{
	rsnano::PortmappingConstantsDto dto;
	auto network_dto{ network_constants.to_dto () };
	if (rsnano::rsn_portmapping_constants_create (&network_dto, &dto) < 0)
		throw std::runtime_error ("could not create portmapping constants");
	lease_duration = std::chrono::seconds (dto.lease_duration_s);
	health_check_period = std::chrono::seconds (dto.health_check_period_s);
}

nano::portmapping_constants::portmapping_constants (rsnano::PortmappingConstantsDto const & dto)
{
	lease_duration = std::chrono::seconds (dto.lease_duration_s);
	health_check_period = std::chrono::seconds (dto.health_check_period_s);
}

rsnano::PortmappingConstantsDto nano::portmapping_constants::to_dto () const
{
	rsnano::PortmappingConstantsDto dto;
	dto.lease_duration_s = lease_duration.count ();
	dto.health_check_period_s = health_check_period.count ();
	return dto;
}

nano::bootstrap_constants::bootstrap_constants (rsnano::BootstrapConstantsDto const & dto)
{
	read_dto (dto);
}

rsnano::BootstrapConstantsDto nano::bootstrap_constants::to_dto () const
{
	rsnano::BootstrapConstantsDto dto;
	dto.lazy_max_pull_blocks = lazy_max_pull_blocks;
	dto.lazy_min_pull_blocks = lazy_min_pull_blocks;
	dto.frontier_retry_limit = frontier_retry_limit;
	dto.lazy_retry_limit = lazy_retry_limit;
	dto.lazy_destinations_retry_limit = lazy_destinations_retry_limit;
	dto.gap_cache_bootstrap_start_interval_ms = gap_cache_bootstrap_start_interval.count ();
	dto.default_frontiers_age_seconds = default_frontiers_age_seconds;
	return dto;
}

void nano::bootstrap_constants::read_dto (rsnano::BootstrapConstantsDto const & dto)
{
	lazy_max_pull_blocks = dto.lazy_max_pull_blocks;
	lazy_min_pull_blocks = dto.lazy_min_pull_blocks;
	frontier_retry_limit = dto.frontier_retry_limit;
	lazy_retry_limit = dto.lazy_retry_limit;
	lazy_destinations_retry_limit = dto.lazy_destinations_retry_limit;
	gap_cache_bootstrap_start_interval = std::chrono::milliseconds (dto.gap_cache_bootstrap_start_interval_ms);
	default_frontiers_age_seconds = dto.default_frontiers_age_seconds;
}

// Create a new random keypair
nano::keypair::keypair ()
{
	rsnano::rsn_keypair_create (prv.bytes.data (), pub.bytes.data ());
}

// Create a keypair given a private key
nano::keypair::keypair (nano::raw_key && prv_a) :
	prv (std::move (prv_a))
{
	rsnano::rsn_keypair_create_from_prv_key (prv.bytes.data (), pub.bytes.data ());
}

// Create a keypair given a hex string of the private key
nano::keypair::keypair (std::string const & prv_a)
{
	rsnano::rsn_keypair_create_from_hex_str (prv_a.c_str (), prv.bytes.data (), pub.bytes.data ());
}

nano::keypair::keypair (nano::raw_key const & priv_key_a, nano::public_key const & pub_key_a) :
	prv (priv_key_a),
	pub (pub_key_a)
{
}

nano::keypair::keypair (const nano::keypair & other_a) :
	prv{ other_a.prv },
	pub{ other_a.pub }
{
}

nano::account_info::account_info () :
	account_info (0, 0, 0, 0, 0, 0, nano::epoch::epoch_0)
{
}

nano::account_info::account_info (nano::block_hash const & head_a, nano::account const & representative_a, nano::block_hash const & open_block_a, nano::amount const & balance_a, uint64_t modified_a, uint64_t block_count_a, nano::epoch epoch_a) :
	handle{ rsnano::rsn_account_info_create (head_a.bytes.data (), representative_a.bytes.data (), open_block_a.bytes.data (), balance_a.bytes.data (), modified_a, block_count_a, static_cast<uint8_t> (epoch_a)) }
{
}

nano::account_info::account_info (nano::account_info const & other_a) :
	handle{ rsnano::rsn_account_info_clone (other_a.handle) }
{
}

nano::account_info::account_info (nano::account_info && other_a) :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::account_info::~account_info ()
{
	if (handle)
		rsnano::rsn_account_info_destroy (handle);
}

nano::account_info & nano::account_info::operator= (nano::account_info const & other_a)
{
	if (handle)
		rsnano::rsn_account_info_destroy (handle);
	handle = rsnano::rsn_account_info_clone (other_a.handle);
	return *this;
}

bool nano::account_info::serialize (nano::stream & stream_a) const
{
	bool success = rsnano::rsn_account_info_serialize (handle, &stream_a);
	return !success;
}

bool nano::account_info::deserialize (nano::stream & stream_a)
{
	bool success = rsnano::rsn_account_info_deserialize (handle, &stream_a);
	return !success;
}

bool nano::account_info::operator== (nano::account_info const & other_a) const
{
	return rsnano::rsn_account_info_equals (handle, other_a.handle);
}

bool nano::account_info::operator!= (nano::account_info const & other_a) const
{
	return !(*this == other_a);
}

size_t nano::account_info::db_size () const
{
	return rsnano::rsn_account_info_db_size ();
}

nano::epoch nano::account_info::epoch () const
{
	rsnano::AccountInfoDto dto;
	rsnano::rsn_account_info_values (handle, &dto);
	return static_cast<nano::epoch> (dto.epoch);
}
nano::block_hash nano::account_info::head () const
{
	rsnano::AccountInfoDto dto;
	rsnano::rsn_account_info_values (handle, &dto);
	nano::block_hash head;
	std::copy (std::begin (dto.head), std::end (dto.head), std::begin (head.bytes));
	return head;
}

nano::account nano::account_info::representative () const
{
	rsnano::AccountInfoDto dto;
	rsnano::rsn_account_info_values (handle, &dto);
	nano::account representative;
	std::copy (std::begin (dto.representative), std::end (dto.representative), std::begin (representative.bytes));
	return representative;
}

nano::block_hash nano::account_info::open_block () const
{
	rsnano::AccountInfoDto dto;
	rsnano::rsn_account_info_values (handle, &dto);
	nano::block_hash open_block;
	std::copy (std::begin (dto.open_block), std::end (dto.open_block), std::begin (open_block.bytes));
	return open_block;
}
nano::amount nano::account_info::balance () const
{
	rsnano::AccountInfoDto dto;
	rsnano::rsn_account_info_values (handle, &dto);
	nano::amount balance;
	std::copy (std::begin (dto.balance), std::end (dto.balance), std::begin (balance.bytes));
	return balance;
}
uint64_t nano::account_info::modified () const
{
	rsnano::AccountInfoDto dto;
	rsnano::rsn_account_info_values (handle, &dto);
	return dto.modified;
}
uint64_t nano::account_info::block_count () const
{
	rsnano::AccountInfoDto dto;
	rsnano::rsn_account_info_values (handle, &dto);
	return dto.block_count;
}

nano::pending_info::pending_info (nano::account const & source_a, nano::amount const & amount_a, nano::epoch epoch_a) :
	source (source_a),
	amount (amount_a),
	epoch (epoch_a)
{
}

bool nano::pending_info::deserialize (nano::stream & stream_a)
{
	auto error (false);
	try
	{
		nano::read (stream_a, source.bytes);
		nano::read (stream_a, amount.bytes);
		nano::read (stream_a, epoch);
	}
	catch (std::runtime_error const &)
	{
		error = true;
	}

	return error;
}

size_t nano::pending_info::db_size () const
{
	return sizeof (source) + sizeof (amount) + sizeof (epoch);
}

bool nano::pending_info::operator== (nano::pending_info const & other_a) const
{
	return source == other_a.source && amount == other_a.amount && epoch == other_a.epoch;
}

nano::pending_key::pending_key (nano::account const & account_a, nano::block_hash const & hash_a) :
	account (account_a),
	hash (hash_a)
{
}

bool nano::pending_key::deserialize (nano::stream & stream_a)
{
	auto error (false);
	try
	{
		nano::read (stream_a, account.bytes);
		nano::read (stream_a, hash.bytes);
	}
	catch (std::runtime_error const &)
	{
		error = true;
	}

	return error;
}

bool nano::pending_key::operator== (nano::pending_key const & other_a) const
{
	return account == other_a.account && hash == other_a.hash;
}

nano::account const & nano::pending_key::key () const
{
	return account;
}

nano::unchecked_info::unchecked_info () :
	handle (rsnano::rsn_unchecked_info_create ())
{
}

nano::unchecked_info::unchecked_info (nano::unchecked_info const & other_a) :
	handle (rsnano::rsn_unchecked_info_clone (other_a.handle))
{
}

nano::unchecked_info::unchecked_info (nano::unchecked_info && other_a) :
	handle (other_a.handle)
{
	other_a.handle = nullptr;
}

nano::unchecked_info::unchecked_info (rsnano::UncheckedInfoHandle * handle_a) :
	handle (handle_a)
{
}

nano::unchecked_info::unchecked_info (std::shared_ptr<nano::block> const & block_a, nano::account const & account_a, nano::signature_verification verified_a) :
	handle (rsnano::rsn_unchecked_info_create2 (block_a->get_handle (), account_a.bytes.data (), static_cast<uint8_t> (verified_a)))
{
}

nano::unchecked_info::unchecked_info (std::shared_ptr<nano::block> const & block) :
	unchecked_info{ block, block->account (), nano::signature_verification::unknown }
{
}

nano::unchecked_info::~unchecked_info ()
{
	if (handle != nullptr)
		rsnano::rsn_unchecked_info_destroy (handle);
}

nano::unchecked_info & nano::unchecked_info::operator= (const nano::unchecked_info & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_unchecked_info_destroy (handle);

	handle = rsnano::rsn_unchecked_info_clone (other_a.handle);
	return *this;
}

std::shared_ptr<nano::block> nano::unchecked_info::get_block () const
{
	auto block_handle = rsnano::rsn_unchecked_info_block (handle);
	return block_handle_to_block (block_handle);
}

nano::account nano::unchecked_info::get_account () const
{
	nano::account account;
	rsnano::rsn_unchecked_info_account (handle, account.bytes.data ());
	return account;
}

nano::signature_verification nano::unchecked_info::get_verified () const
{
	return static_cast<nano::signature_verification> (rsnano::rsn_unchecked_info_verified (handle));
}

void nano::unchecked_info::set_verified (nano::signature_verification verified)
{
	rsnano::rsn_unchecked_info_verified_set (handle, static_cast<uint8_t> (verified));
}

void nano::unchecked_info::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_unchecked_info_serialize (handle, &stream_a))
		throw std::runtime_error ("could not serialize unchecked_info");
}

bool nano::unchecked_info::deserialize (nano::stream & stream_a)
{
	auto success = rsnano::rsn_unchecked_info_deserialize (handle, &stream_a);
	return !success;
}

uint64_t nano::unchecked_info::modified () const
{
	return rsnano::rsn_unchecked_info_modified (handle);
}

nano::endpoint_key::endpoint_key (std::array<uint8_t, 16> const & address_a, uint16_t port_a) :
	address (address_a), network_port (boost::endian::native_to_big (port_a))
{
}

std::array<uint8_t, 16> const & nano::endpoint_key::address_bytes () const
{
	return address;
}

uint16_t nano::endpoint_key::port () const
{
	return boost::endian::big_to_native (network_port);
}

nano::confirmation_height_info::confirmation_height_info ()
{
	rsnano::rsn_confirmation_height_info_create (&dto);
}

uint64_t nano::confirmation_height_info::height () const
{
	return dto.height;
}

nano::block_hash nano::confirmation_height_info::frontier () const
{
	nano::block_hash hash;
	std::copy (std::begin (dto.frontier), std::end (dto.frontier), std::begin (hash.bytes));
	return hash;
}

nano::confirmation_height_info::confirmation_height_info (uint64_t confirmation_height_a, nano::block_hash const & confirmed_frontier_a)
{
	rsnano::rsn_confirmation_height_info_create2 (confirmation_height_a, confirmed_frontier_a.bytes.data (), &dto);
}

void nano::confirmation_height_info::serialize (nano::stream & stream_a) const
{
	if (!rsnano::rsn_confirmation_height_info_serialize (&dto, &stream_a))
	{
		throw std::runtime_error ("could not serialize confirmation_height_info");
	}
}

bool nano::confirmation_height_info::deserialize (nano::stream & stream_a)
{
	bool success = rsnano::rsn_confirmation_height_info_deserialize (&dto, &stream_a);
	return !success;
}

nano::block_info::block_info (nano::account const & account_a, nano::amount const & balance_a) :
	account (account_a),
	balance (balance_a)
{
}

bool nano::vote::operator== (nano::vote const & other_a) const
{
	return rsnano::rsn_vote_equals (handle, other_a.handle);
}

bool nano::vote::operator!= (nano::vote const & other_a) const
{
	return !(*this == other_a);
}

std::vector<nano::block_hash> read_block_hashes (rsnano::VoteHandle const * handle)
{
	auto hashes_dto{ rsnano::rsn_vote_hashes (handle) };
	std::vector<nano::block_hash> hashes;
	hashes.resize (hashes_dto.count);
	for (auto i (0); i < hashes_dto.count; ++i)
	{
		std::copy (std::begin (hashes_dto.hashes[i]), std::end (hashes_dto.hashes[i]), std::begin (hashes[i].bytes));
	}
	rsnano::rsn_vote_hashes_destroy (hashes_dto.handle);
	return hashes;
}

void nano::vote::serialize_json (boost::property_tree::ptree & tree) const
{
	rsnano::rsn_vote_serialize_json (handle, &tree);
}

/**
 * Returns the timestamp of the vote (with the duration bits masked, set to zero)
 * If it is a final vote, all the bits including duration bits are returned as they are, all FF
 */
uint64_t nano::vote::timestamp () const
{
	return rsnano::rsn_vote_timestamp (handle);
}

uint8_t nano::vote::duration_bits () const
{
	return rsnano::rsn_vote_duration_bits (handle);
}

std::chrono::milliseconds nano::vote::duration () const
{
	return std::chrono::milliseconds{ rsnano::rsn_vote_duration_ms (handle) };
}

std::vector<nano::block_hash> nano::vote::hashes () const
{
	auto hashes{ read_block_hashes (handle) };
	return hashes;
}

nano::vote::vote () :
	handle (rsnano::rsn_vote_create ())
{
}

nano::vote::vote (rsnano::VoteHandle * handle_a) :
	handle (handle_a)
{
}

nano::vote::vote (nano::vote const & other_a) :
	handle (rsnano::rsn_vote_copy (other_a.handle))
{
}

nano::vote::vote (nano::vote && other_a) :
	handle (other_a.handle)
{
	other_a.handle = nullptr;
}

nano::vote::vote (nano::account const & account) :
	handle (rsnano::rsn_vote_create ())
{
	rsnano::rsn_vote_account_set (handle, account.bytes.data ());
}

nano::vote::vote (bool & error_a, nano::stream & stream_a) :
	handle{ rsnano::rsn_vote_create () }
{
	error_a = deserialize (stream_a);
}

nano::vote::vote (nano::account const & account_a, nano::raw_key const & prv_a, uint64_t timestamp_a, uint8_t duration, std::vector<nano::block_hash> const & hashes)
{
	handle = rsnano::rsn_vote_create2 (account_a.bytes.data (), prv_a.bytes.data (), timestamp_a, duration, reinterpret_cast<const uint8_t (*)[32]> (hashes.data ()), hashes.size ());
}

nano::vote::~vote ()
{
	if (handle != nullptr)
	{
		rsnano::rsn_vote_destroy (handle);
	}
}

std::string nano::vote::hashes_string () const
{
	auto dto{ rsnano::rsn_vote_hashes_string (handle) };
	return rsnano::convert_dto_to_string (dto);
}

std::string const nano::vote::hash_prefix = "vote ";

nano::block_hash nano::vote::hash () const
{
	nano::block_hash result;
	rsnano::rsn_vote_hash (handle, result.bytes.data ());
	return result;
}

nano::block_hash nano::vote::full_hash () const
{
	nano::block_hash result;
	rsnano::rsn_vote_full_hash (handle, result.bytes.data ());
	return result;
}

void nano::vote::serialize (nano::stream & stream_a) const
{
	auto result = rsnano::rsn_vote_serialize (handle, &stream_a);
	if (result != 0)
	{
		throw std::runtime_error ("Could not serialize vote");
	}
}

bool nano::vote::deserialize (nano::stream & stream_a)
{
	auto error = rsnano::rsn_vote_deserialize (handle, &stream_a) != 0;
	return error;
}

bool nano::vote::validate () const
{
	return rsnano::rsn_vote_validate (handle);
}

nano::account nano::vote::account () const
{
	nano::account account;
	rsnano::rsn_vote_account (handle, account.bytes.data ());
	return account;
}

nano::signature nano::vote::signature () const
{
	nano::signature signature;
	rsnano::rsn_vote_signature (handle, signature.bytes.data ());
	return signature;
}

void nano::vote::flip_signature_bit_0 ()
{
	nano::signature signature;
	rsnano::rsn_vote_signature (handle, signature.bytes.data ());
	signature.bytes[0] ^= 1;
	rsnano::rsn_vote_signature_set (handle, signature.bytes.data ());
}

rsnano::VoteHandle * nano::vote::get_handle () const
{
	return handle;
}

const void * nano::vote::get_rust_data_pointer () const
{
	return rsnano::rsn_vote_rust_data_pointer (handle);
}

nano::block_hash nano::iterate_vote_blocks_as_hash::operator() (nano::block_hash const & item) const
{
	return item;
}

nano::vote_uniquer::vote_uniquer (nano::block_uniquer & uniquer_a) :
	handle (rsnano::rsn_vote_uniquer_create ())
{
}

nano::vote_uniquer::~vote_uniquer ()
{
	if (handle != nullptr)
	{
		rsnano::rsn_vote_uniquer_destroy (handle);
	}
}

std::shared_ptr<nano::vote> nano::vote_uniquer::unique (std::shared_ptr<nano::vote> const & vote_a)
{
	if (vote_a == nullptr)
	{
		return nullptr;
	}
	auto uniqued (rsnano::rsn_vote_uniquer_unique (handle, vote_a->get_handle ()));
	if (uniqued == vote_a->get_handle ())
	{
		return vote_a;
	}
	else
	{
		return std::make_shared<nano::vote> (uniqued);
	}
}

size_t nano::vote_uniquer::size ()
{
	return rsnano::rsn_vote_uniquer_size (handle);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (vote_uniquer & vote_uniquer, std::string const & name)
{
	auto count = vote_uniquer.size ();
	auto sizeof_element = sizeof (vote_uniquer::value_type);
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "votes", count, sizeof_element }));
	return composite;
}

nano::wallet_id nano::random_wallet_id ()
{
	nano::wallet_id wallet_id;
	rsnano::rsn_random_wallet_id (wallet_id.bytes.data ());
	return wallet_id;
}

nano::unchecked_key::unchecked_key (nano::hash_or_account const & dependency) :
	unchecked_key{ dependency, 0 }
{
}

nano::unchecked_key::unchecked_key (nano::hash_or_account const & previous_a, nano::block_hash const & hash_a) :
	previous (previous_a.as_block_hash ()),
	hash (hash_a)
{
}

nano::unchecked_key::unchecked_key (nano::uint512_union const & union_a) :
	previous (union_a.uint256s[0].number ()),
	hash (union_a.uint256s[1].number ())
{
}

bool nano::unchecked_key::deserialize (nano::stream & stream_a)
{
	auto error (false);
	try
	{
		nano::read (stream_a, previous.bytes);
		nano::read (stream_a, hash.bytes);
	}
	catch (std::runtime_error const &)
	{
		error = true;
	}

	return error;
}

bool nano::unchecked_key::operator== (nano::unchecked_key const & other_a) const
{
	return previous == other_a.previous && hash == other_a.hash;
}

bool nano::unchecked_key::operator< (nano::unchecked_key const & other_a) const
{
	return previous != other_a.previous ? previous < other_a.previous : hash < other_a.hash;
}

nano::block_hash const & nano::unchecked_key::key () const
{
	return previous;
}
rsnano::UncheckedKeyDto nano::unchecked_key::to_dto () const
{
	rsnano::UncheckedKeyDto dto;
	std::copy (std::begin (previous.bytes), std::end (previous.bytes), std::begin (dto.previous));
	std::copy (std::begin (hash.bytes), std::end (hash.bytes), std::begin (dto.hash));
	return dto;
}

nano::generate_cache::generate_cache () :
	handle{ rsnano::rsn_generate_cache_create () }
{
}

nano::generate_cache::generate_cache (rsnano::GenerateCacheHandle * handle_a) :
	handle{ handle_a }
{
}

void nano::generate_cache::enable_all ()
{
	rsnano::rsn_generate_cache_enable_all (handle);
}

nano::generate_cache::generate_cache (nano::generate_cache && other_a) noexcept :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::generate_cache::generate_cache (const nano::generate_cache & other_a) :
	handle{ rsnano::rsn_generate_cache_clone (other_a.handle) }
{
}

nano::generate_cache::~generate_cache ()
{
	if (handle)
		rsnano::rsn_generate_cache_destroy (handle);
}

nano::generate_cache & nano::generate_cache::operator= (nano::generate_cache && other_a)
{
	if (handle != nullptr)
		rsnano::rsn_generate_cache_destroy (handle);
	handle = other_a.handle;
	other_a.handle = nullptr;
	return *this;
}
nano::generate_cache & nano::generate_cache::operator= (const nano::generate_cache & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_generate_cache_destroy (handle);
	handle = rsnano::rsn_generate_cache_clone (other_a.handle);
	return *this;
}
bool nano::generate_cache::reps () const
{
	return rsnano::rsn_generate_cache_reps (handle);
}
void nano::generate_cache::enable_reps (bool enable)
{
	rsnano::rsn_generate_cache_set_reps (handle, enable);
}
bool nano::generate_cache::cemented_count () const
{
	return rsnano::rsn_generate_cache_cemented_count (handle);
}
void nano::generate_cache::enable_cemented_count (bool enable)
{
	rsnano::rsn_generate_cache_set_cemented_count (handle, enable);
}
void nano::generate_cache::enable_unchecked_count (bool enable)
{
	rsnano::rsn_generate_cache_set_unchecked_count (handle, enable);
}
bool nano::generate_cache::account_count () const
{
	return rsnano::rsn_generate_cache_account_count (handle);
}
void nano::generate_cache::enable_account_count (bool enable)
{
	rsnano::rsn_generate_cache_set_account_count (handle, enable);
}
bool nano::generate_cache::block_count () const
{
	return rsnano::rsn_generate_cache_block_count (handle);
}
void nano::generate_cache::enable_block_count (bool enable)
{
	rsnano::rsn_generate_cache_set_account_count (handle, enable);
}

nano::stat::detail nano::to_stat_detail (nano::process_result process_result)
{
	nano::stat::detail result;
	switch (process_result)
	{
		case process_result::progress:
			return nano::stat::detail::progress;
			break;
		case process_result::bad_signature:
			return nano::stat::detail::bad_signature;
			break;
		case process_result::old:
			return nano::stat::detail::old;
			break;
		case process_result::negative_spend:
			return nano::stat::detail::negative_spend;
			break;
		case process_result::fork:
			return nano::stat::detail::fork;
			break;
		case process_result::unreceivable:
			return nano::stat::detail::unreceivable;
			break;
		case process_result::gap_previous:
			return nano::stat::detail::gap_previous;
			break;
		case process_result::gap_source:
			return nano::stat::detail::gap_source;
			break;
		case process_result::gap_epoch_open_pending:
			return nano::stat::detail::gap_epoch_open_pending;
			break;
		case process_result::opened_burn_account:
			return nano::stat::detail::opened_burn_account;
			break;
		case process_result::balance_mismatch:
			return nano::stat::detail::balance_mismatch;
			break;
		case process_result::representative_mismatch:
			return nano::stat::detail::representative_mismatch;
			break;
		case process_result::block_position:
			return nano::stat::detail::block_position;
			break;
		case process_result::insufficient_work:
			return nano::stat::detail::insufficient_work;
			break;
	}
	return result;
}

nano::ledger_cache::ledger_cache () :
	handle{ rsnano::rsn_ledger_cache_create () }, rep_weights_m{ rsnano::rsn_ledger_cache_weights (handle) }
{
}

nano::ledger_cache::ledger_cache (rsnano::LedgerCacheHandle * handle_a) :
	handle{ handle_a }, rep_weights_m{ rsnano::rsn_ledger_cache_weights (handle) }
{
}

nano::ledger_cache::ledger_cache (ledger_cache && other_a) :
	handle{ other_a.handle }, rep_weights_m{ rsnano::rsn_ledger_cache_weights (handle) }
{
	other_a.handle = nullptr;
}

nano::ledger_cache::~ledger_cache ()
{
	if (handle != nullptr)
		rsnano::rsn_ledger_cache_destroy (handle);
}

nano::ledger_cache & nano::ledger_cache::operator= (nano::ledger_cache && other_a)
{
	if (handle != nullptr)
		rsnano::rsn_ledger_cache_destroy (handle);
	handle = other_a.handle;
	other_a.handle = nullptr;
	rep_weights_m = std::move (other_a.rep_weights_m);
	return *this;
}

nano::rep_weights & nano::ledger_cache::rep_weights ()
{
	return rep_weights_m;
}
uint64_t nano::ledger_cache::cemented_count () const
{
	return rsnano::rsn_ledger_cache_cemented_count (handle);
}
uint64_t nano::ledger_cache::block_count () const
{
	return rsnano::rsn_ledger_cache_block_count (handle);
}
uint64_t nano::ledger_cache::pruned_count () const
{
	return rsnano::rsn_ledger_cache_pruned_count (handle);
}
uint64_t nano::ledger_cache::account_count () const
{
	return rsnano::rsn_ledger_cache_account_count (handle);
}
bool nano::ledger_cache::final_votes_confirmation_canary () const
{
	return rsnano::rsn_ledger_cache_final_votes_confirmation_canary (handle);
}
void nano::ledger_cache::add_cemented (uint64_t count)
{
	rsnano::rsn_ledger_cache_add_cemented (handle, count);
}
void nano::ledger_cache::add_blocks (uint64_t count)
{
	rsnano::rsn_ledger_cache_add_blocks (handle, count);
}
void nano::ledger_cache::add_pruned (uint64_t count)
{
	rsnano::rsn_ledger_cache_add_pruned (handle, count);
}
void nano::ledger_cache::add_accounts (uint64_t count)
{
	rsnano::rsn_ledger_cache_add_accounts (handle, count);
}
void nano::ledger_cache::set_final_votes_confirmation_canary (bool canary)
{
	rsnano::rsn_ledger_cache_set_final_votes_confirmation_canary (handle, canary);
}
void nano::ledger_cache::remove_blocks (uint64_t count)
{
	rsnano::rsn_ledger_cache_remove_blocks (handle, count);
}
void nano::ledger_cache::remove_accounts (uint64_t count)
{
	rsnano::rsn_ledger_cache_remove_accounts (handle, count);
}

nano::election_status::election_status () :
	handle (rsnano::rsn_election_status_create ())
{
}
nano::election_status::election_status (std::shared_ptr<nano::block> const & winner_a) :
	handle (rsnano::rsn_election_status_create1 (winner_a->get_handle ()))
{
}

nano::election_status::election_status (nano::election_status const & other_a) :
	handle (rsnano::rsn_election_status_clone (other_a.handle))
{
}

nano::election_status::~election_status ()
{
	if (handle != nullptr)
		rsnano::rsn_election_status_destroy (handle);
}

nano::election_status & nano::election_status::operator= (const nano::election_status & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_election_status_destroy (handle);

	handle = rsnano::rsn_election_status_clone (other_a.handle);
	return *this;
}

std::shared_ptr<nano::block> nano::election_status::get_winner () const
{
	auto block_handle = rsnano::rsn_election_status_get_winner (handle);
	return block_handle_to_block (block_handle);
}

nano::amount nano::election_status::get_tally () const
{
	nano::amount tally;
	rsnano::rsn_election_status_get_tally (handle, tally.bytes.data ());
	return tally;
}

nano::amount nano::election_status::get_final_tally () const
{
	nano::amount final_tally;
	rsnano::rsn_election_status_get_final_tally (handle, final_tally.bytes.data ());
	return final_tally;
}

std::chrono::milliseconds nano::election_status::get_election_end () const
{
	return std::chrono::milliseconds (rsnano::rsn_election_status_get_election_end (handle));
}

std::chrono::milliseconds nano::election_status::get_election_duration () const
{
	return std::chrono::milliseconds (rsnano::rsn_election_status_get_election_duration (handle));
}

unsigned nano::election_status::get_confirmation_request_count () const
{
	return rsnano::rsn_election_status_get_confirmation_request_count (handle);
}

unsigned nano::election_status::get_block_count () const
{
	return rsnano::rsn_election_status_get_block_count (handle);
}

unsigned nano::election_status::get_voter_count () const
{
	return rsnano::rsn_election_status_get_vote_count (handle);
}

nano::election_status_type nano::election_status::get_election_status_type () const
{
	return static_cast<nano::election_status_type> (rsnano::rsn_election_status_get_election_status_type (handle));
}

void nano::election_status::set_winner (std::shared_ptr<nano::block> winner)
{
	auto block_handle = winner == nullptr ? nullptr : winner->get_handle ();
	rsnano::rsn_election_status_set_winner (handle, block_handle);
}

void nano::election_status::set_tally (nano::amount tally)f
{
	rsnano::rsn_election_status_set_tally (handle, tally.bytes.data ());
}

void nano::election_status::set_final_tally (nano::amount final_tally)
{
	rsnano::rsn_election_status_set_final_tally (handle, final_tally.bytes.data ());
}

void nano::election_status::set_block_count (uint32_t block_count)
{
	rsnano::rsn_election_status_set_block_count (handle, block_count);
}

void nano::election_status::set_voter_count (uint32_t voter_count)
{
	rsnano::rsn_election_status_set_voter_count (handle, voter_count);
}

void nano::election_status::set_confirmation_request_count (uint32_t confirmation_request_count)
{
	rsnano::rsn_election_status_set_confirmation_request_count (handle, confirmation_request_count);
}

void nano::election_status::set_election_end (std::chrono::milliseconds election_end)
{
	rsnano::rsn_election_status_set_election_end (handle, election_end.count ());
}

void nano::election_status::set_election_duration (std::chrono::milliseconds election_duration)
{
	rsnano::rsn_election_status_set_election_duration (handle, election_duration.count ());
}

void nano::election_status::set_election_status_type (nano::election_status_type election_status_type)
{
	rsnano::rsn_election_status_set_election_status_type (handle, static_cast<uint8_t> (election_status_type));
}