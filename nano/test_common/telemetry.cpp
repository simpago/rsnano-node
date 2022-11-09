#include <nano/node/common.hpp>
#include <nano/node/messages.hpp>
#include <nano/test_common/telemetry.hpp>

#include <gtest/gtest.h>

void nano::test::compare_default_telemetry_response_data_excluding_signature (nano::telemetry_data const & telemetry_data_a, nano::network_params const & network_params_a, uint64_t bandwidth_limit_a, uint64_t active_difficulty_a)
{
	ASSERT_EQ (telemetry_data_a.get_block_count (), 1);
	ASSERT_EQ (telemetry_data_a.get_cemented_count (), 1);
	ASSERT_EQ (telemetry_data_a.get_bandwidth_cap (), bandwidth_limit_a);
	ASSERT_EQ (telemetry_data_a.get_peer_count (), 1);
	ASSERT_EQ (telemetry_data_a.get_protocol_version (), network_params_a.network.protocol_version);
	ASSERT_EQ (telemetry_data_a.get_unchecked_count (), 0);
	ASSERT_EQ (telemetry_data_a.get_account_count (), 1);
	ASSERT_LT (telemetry_data_a.get_uptime (), 100);
	ASSERT_EQ (telemetry_data_a.get_genesis_block (), network_params_a.ledger.genesis->hash ());
	ASSERT_EQ (telemetry_data_a.get_major_version (), nano::get_major_node_version ());
	ASSERT_EQ (telemetry_data_a.get_minor_version (), nano::get_minor_node_version ());
	ASSERT_EQ (telemetry_data_a.get_patch_version (), nano::get_patch_node_version ());
	ASSERT_EQ (telemetry_data_a.get_pre_release_version (), nano::get_pre_release_node_version ());
	ASSERT_EQ (telemetry_data_a.get_maker (), static_cast<std::underlying_type_t<nano::telemetry_maker>> (nano::telemetry_maker::nf_node));
	ASSERT_GT (telemetry_data_a.get_timestamp (), std::chrono::system_clock::now () - std::chrono::seconds (100));
	ASSERT_EQ (telemetry_data_a.get_active_difficulty (), active_difficulty_a);
	ASSERT_EQ (telemetry_data_a.get_unknown_data (), std::vector<uint8_t>{});
}

void nano::test::compare_default_telemetry_response_data (nano::telemetry_data const & telemetry_data_a, nano::network_params const & network_params_a, uint64_t bandwidth_limit_a, uint64_t active_difficulty_a, nano::keypair const & node_id_a)
{
	ASSERT_FALSE (telemetry_data_a.validate_signature ());
	compare_default_telemetry_response_data_excluding_signature (telemetry_data_a, network_params_a, bandwidth_limit_a, active_difficulty_a);
	ASSERT_EQ (telemetry_data_a.get_node_id (), node_id_a.pub);
}
