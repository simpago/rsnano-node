#include <nano/lib/blocks.hpp>
#include <nano/lib/config.hpp>

#include <boost/filesystem/path.hpp>
#include <boost/format.hpp>
#include <boost/lexical_cast.hpp>

#include <valgrind/valgrind.h>

namespace
{
// useful for boost_lexical cast to allow conversion of hex strings
template <typename ElemT>
struct HexTo
{
	ElemT value;
	operator ElemT () const
	{
		return value;
	}
	friend std::istream & operator>> (std::istream & in, HexTo & out)
	{
		in >> std::hex >> out.value;
		return in;
	}
};
} // namespace

nano::work_thresholds::work_thresholds (uint64_t epoch_1_a, uint64_t epoch_2_a, uint64_t epoch_2_receive_a)
{
	rsnano::rsn_work_thresholds_create (&dto, epoch_1_a, epoch_2_a, epoch_2_receive_a);
}

nano::work_thresholds::work_thresholds (rsnano::WorkThresholdsDto const & dto_a) :
	dto (dto_a)
{
}

nano::work_thresholds const nano::work_thresholds::publish_full ()
{
	rsnano::WorkThresholdsDto dto;
	rsnano::rsn_work_thresholds_publish_full (&dto);
	return nano::work_thresholds (dto);
}

nano::work_thresholds const nano::work_thresholds::publish_beta ()
{
	rsnano::WorkThresholdsDto dto;
	rsnano::rsn_work_thresholds_publish_beta (&dto);
	return nano::work_thresholds (dto);
}

nano::work_thresholds const nano::work_thresholds::publish_dev ()
{
	rsnano::WorkThresholdsDto dto;
	rsnano::rsn_work_thresholds_publish_dev (&dto);
	return nano::work_thresholds (dto);
}

nano::work_thresholds const nano::work_thresholds::publish_test ()
{
	rsnano::WorkThresholdsDto dto;
	rsnano::rsn_work_thresholds_publish_test (&dto);
	return nano::work_thresholds (dto);
}

uint64_t nano::work_thresholds::get_base () const
{
	return dto.base;
}

uint64_t nano::work_thresholds::get_epoch_2 () const
{
	return dto.epoch_2;
}

uint64_t nano::work_thresholds::get_epoch_2_receive () const
{
	return dto.epoch_2_receive;
}

uint64_t nano::work_thresholds::get_entry () const
{
	return dto.entry;
}

uint64_t nano::work_thresholds::get_epoch_1 () const
{
	return dto.epoch_1;
}

uint8_t work_version_to_uint8 (nano::work_version const version_a)
{
	switch (version_a)
	{
		case nano::work_version::unspecified:
			return 0;
		case nano::work_version::work_1:
			return 1;
		default:
			return std::numeric_limits<uint8_t>::max ();
	}
}

uint64_t nano::work_thresholds::threshold_entry (nano::work_version const version_a, nano::block_type const type_a) const
{
	return rsnano::rsn_work_thresholds_threshold_entry (&dto, work_version_to_uint8 (version_a), static_cast<uint8_t> (type_a));
}

#ifndef NANO_FUZZER_TEST
uint64_t nano::work_thresholds::value (nano::root const & root_a, uint64_t work_a) const
{
	uint8_t bytes[32];
	std::copy (std::begin (root_a.bytes), std::end (root_a.bytes), std::begin (bytes));
	return rsnano::rsn_work_thresholds_value (&dto, &bytes, work_a);
}
#else
uint64_t nano::work_thresholds::value (nano::root const & root_a, uint64_t work_a) const
{
	return dto.base + 1;
}
#endif

uint64_t nano::work_thresholds::threshold (nano::block_details const & details_a) const
{
	return rsnano::rsn_work_thresholds_threshold (&dto, &details_a.dto);
}

uint64_t nano::work_thresholds::threshold (nano::work_version const version_a, nano::block_details const details_a) const
{
	return rsnano::rsn_work_thresholds_threshold2 (&dto, work_version_to_uint8 (version_a), &details_a.dto);
}

double nano::work_thresholds::normalized_multiplier (double const multiplier_a, uint64_t const threshold_a) const
{
	return rsnano::rsn_work_thresholds_normalized_multiplier (&dto, multiplier_a, threshold_a);
}

double nano::work_thresholds::denormalized_multiplier (double const multiplier_a, uint64_t const threshold_a) const
{
	return rsnano::rsn_work_thresholds_denormalized_multiplier (&dto, multiplier_a, threshold_a);
}

uint64_t nano::work_thresholds::threshold_base (nano::work_version const version_a) const
{
	return rsnano::rsn_work_thresholds_threshold_base (&dto, work_version_to_uint8 (version_a));
}

uint64_t nano::work_thresholds::difficulty (nano::work_version const version_a, nano::root const & root_a, uint64_t const work_a) const
{
	return rsnano::rsn_work_thresholds_difficulty (&dto, work_version_to_uint8 (version_a), root_a.bytes.data (), work_a);
}

uint64_t nano::work_thresholds::difficulty (nano::block const & block_a) const
{
	return rsnano::rsn_work_thresholds_difficulty_block (&dto, block_a.get_handle ());
}

bool nano::work_thresholds::validate_entry (nano::work_version const version_a, nano::root const & root_a, uint64_t const work_a) const
{
	return rsnano::rsn_work_thresholds_validate_entry (&dto, work_version_to_uint8 (version_a), root_a.bytes.data (), work_a);
}

bool nano::work_thresholds::validate_entry (nano::block const & block_a) const
{
	return rsnano::rsn_work_thresholds_validate_entry_block (&dto, block_a.get_handle ());
}

nano::networks nano::network_constants::active_network ()
{
	return static_cast<nano::networks> (rsnano::rsn_network_constants_active_network ());
}

void nano::network_constants::set_active_network (nano::networks network_a)
{
	rsnano::rsn_network_constants_active_network_set (static_cast<uint16_t> (network_a));
}

nano::network_constants::network_constants (nano::work_thresholds work_a, nano::networks network_a) :
	work (nano::work_thresholds (0, 0, 0))
{
	rsnano::NetworkConstantsDto dto;
	if (rsnano::rsn_network_constants_create (&dto, &work_a.dto, static_cast<uint16_t> (network_a)) < 0)
	{
		throw std::runtime_error ("could not create network constants");
	}

	read_dto (dto);
}

nano::network_constants::network_constants (rsnano::NetworkConstantsDto const & dto) :
	work (nano::work_thresholds (0, 0, 0))
{
	read_dto (dto);
}

void nano::network_constants::read_dto (rsnano::NetworkConstantsDto const & dto)
{
	work = nano::work_thresholds (dto.work);
	current_network = static_cast<nano::networks> (dto.current_network);
	protocol_version = dto.protocol_version;
	protocol_version_min = dto.protocol_version_min;
	principal_weight_factor = dto.principal_weight_factor;
	default_node_port = dto.default_node_port;
	default_rpc_port = dto.default_rpc_port;
	default_ipc_port = dto.default_ipc_port;
	default_websocket_port = dto.default_websocket_port;
	request_interval_ms = dto.request_interval_ms;
	cleanup_period = std::chrono::seconds (dto.cleanup_period_s);
	idle_timeout = std::chrono::seconds (dto.idle_timeout_s);
	syn_cookie_cutoff = std::chrono::seconds (dto.sync_cookie_cutoff_s);
	bootstrap_interval = std::chrono::seconds (dto.bootstrap_interval_s);
	max_peers_per_ip = dto.max_peers_per_ip;
	max_peers_per_subnetwork = dto.max_peers_per_subnetwork;
	peer_dump_interval = std::chrono::seconds (dto.peer_dump_interval_s);
	ipv6_subnetwork_prefix_for_limiting = dto.ipv6_subnetwork_prefix_for_limiting;
	silent_connection_tolerance_time = std::chrono::seconds (dto.silent_connection_tolerance_time_s);
}

bool nano::network_constants::set_active_network (std::string network_a)
{
	return rsnano::rsn_network_constants_active_network_set_str (network_a.c_str ()) < 0;
}

std::chrono::milliseconds nano::network_constants::cleanup_period_half () const
{
	auto dto{ to_dto () };
	return std::chrono::milliseconds (rsnano::rsn_network_constants_cleanup_period_half_ms (&dto));
}

std::chrono::seconds nano::network_constants::cleanup_cutoff () const
{
	auto dto{ to_dto () };
	return std::chrono::seconds (rsnano::rsn_network_constants_cleanup_cutoff_s (&dto));
}

nano::networks nano::network_constants::network () const
{
	return current_network;
}

char const * nano::network_constants::get_current_network_as_string ()
{
	return is_live_network () ? "live" : is_beta_network () ? "beta"
	: is_test_network ()                                    ? "test"
															: "dev";
}

bool nano::network_constants::is_live_network () const
{
	// return current_network == nano::networks::nano_live_network;
	auto dto{ to_dto () };
	return rsnano::rsn_network_constants_is_live_network (&dto);
}

bool nano::network_constants::is_beta_network () const
{
	// return current_network == nano::networks::nano_beta_network;
	auto dto{ to_dto () };
	return rsnano::rsn_network_constants_is_beta_network (&dto);
}

bool nano::network_constants::is_dev_network () const
{
	// return current_network == nano::networks::nano_dev_network;
	auto dto{ to_dto () };
	return rsnano::rsn_network_constants_is_dev_network (&dto);
}

bool nano::network_constants::is_test_network () const
{
	// return current_network == nano::networks::nano_test_network;
	auto dto{ to_dto () };
	return rsnano::rsn_network_constants_is_test_network (&dto);
}

rsnano::NetworkConstantsDto nano::network_constants::to_dto () const
{
	rsnano::NetworkConstantsDto dto;
	dto.current_network = static_cast<uint16_t> (current_network);
	dto.work = work.dto;
	dto.principal_weight_factor = principal_weight_factor;
	dto.default_node_port = default_node_port;
	dto.default_rpc_port = default_rpc_port;
	dto.default_ipc_port = default_ipc_port;
	dto.protocol_version_min = protocol_version_min;
	dto.default_websocket_port = default_websocket_port;
	dto.request_interval_ms = request_interval_ms;
	dto.cleanup_period_s = cleanup_period.count ();
	dto.idle_timeout_s = idle_timeout.count ();
	dto.sync_cookie_cutoff_s = syn_cookie_cutoff.count ();
	dto.bootstrap_interval_s = bootstrap_interval.count ();
	dto.max_peers_per_ip = max_peers_per_ip;
	dto.max_peers_per_subnetwork = max_peers_per_subnetwork;
	dto.peer_dump_interval_s = peer_dump_interval.count ();
	dto.protocol_version = protocol_version;
	dto.protocol_version_min = protocol_version_min;
	return dto;
}

namespace nano
{
char const * network_constants::active_network_err_msg = "Invalid network. Valid values are live, test, beta and dev.";

uint8_t get_major_node_version ()
{
	return boost::numeric_cast<uint8_t> (boost::lexical_cast<int> (NANO_MAJOR_VERSION_STRING));
}
uint8_t get_minor_node_version ()
{
	return boost::numeric_cast<uint8_t> (boost::lexical_cast<int> (NANO_MINOR_VERSION_STRING));
}
uint8_t get_patch_node_version ()
{
	return boost::numeric_cast<uint8_t> (boost::lexical_cast<int> (NANO_PATCH_VERSION_STRING));
}
uint8_t get_pre_release_node_version ()
{
	return boost::numeric_cast<uint8_t> (boost::lexical_cast<int> (NANO_PRE_RELEASE_VERSION_STRING));
}

uint64_t get_env_threshold_or_default (char const * variable_name, uint64_t const default_value)
{
	auto * value = getenv (variable_name);
	return value ? boost::lexical_cast<HexTo<uint64_t>> (value) : default_value;
}

uint16_t test_node_port ()
{
	return rsnano::rsn_test_node_port ();
}

std::array<uint8_t, 2> test_magic_number ()
{
	auto test_env = get_env_or_default ("NANO_TEST_MAGIC_NUMBER", "RX");
	std::array<uint8_t, 2> ret;
	std::copy (test_env.begin (), test_env.end (), ret.data ());
	return ret;
}

void force_nano_dev_network ()
{
	nano::network_constants::set_active_network (nano::networks::nano_dev_network);
}

bool running_within_valgrind ()
{
	return (RUNNING_ON_VALGRIND > 0);
}

bool memory_intensive_instrumentation ()
{
	return is_tsan_build () || nano::running_within_valgrind ();
}

bool slow_instrumentation ()
{
	return is_tsan_build () || nano::running_within_valgrind ();
}

bool is_sanitizer_build ()
{
	return is_asan_build () || is_tsan_build ();
}

std::string get_node_toml_config_path (boost::filesystem::path const & data_path)
{
	return (data_path / "config-node.toml").string ();
}

std::string get_rpc_toml_config_path (boost::filesystem::path const & data_path)
{
	return (data_path / "config-rpc.toml").string ();
}

std::string get_qtwallet_toml_config_path (boost::filesystem::path const & data_path)
{
	return (data_path / "config-qtwallet.toml").string ();
}

std::string get_access_toml_config_path (boost::filesystem::path const & data_path)
{
	return (data_path / "config-access.toml").string ();
}

std::string get_tls_toml_config_path (boost::filesystem::path const & data_path)
{
	return (data_path / "config-tls.toml").string ();
}
} // namespace nano

std::optional<std::string> nano::get_env (const char * variable_name)
{
	auto value = std::getenv (variable_name);
	if (value)
	{
		return value;
	}
	return {};
}

std::string nano::get_env_or_default (char const * variable_name, std::string default_value)
{
	auto value = nano::get_env (variable_name);
	return value ? *value : default_value;
}

int nano::get_env_int_or_default (const char * variable_name, const int default_value)
{
	auto value = nano::get_env (variable_name);
	if (value)
	{
		try
		{
			return boost::lexical_cast<int> (*value);
		}
		catch (...)
		{
			// It is unexpected that this exception will be caught, log to cerr the reason.
			std::cerr << boost::str (boost::format ("Error parsing environment variable: %1% value: %2%") % variable_name % *value);
			throw;
		}
	}
	return default_value;
}

uint32_t nano::test_scan_wallet_reps_delay ()
{
	auto test_env = nano::get_env_or_default ("NANO_TEST_WALLET_SCAN_REPS_DELAY", "900000"); // 15 minutes by default
	return boost::lexical_cast<uint32_t> (test_env);
}
