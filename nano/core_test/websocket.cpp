#include <nano/core_test/fakes/websocket_client.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/websocket.hpp>
#include <nano/test_common/network.hpp>
#include <nano/test_common/system.hpp>
#include <nano/test_common/telemetry.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <boost/property_tree/json_parser.hpp>

#include <chrono>
#include <cstdlib>
#include <future>
#include <memory>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

using namespace std::chrono_literals;

// Tests updating options of block confirmations
TEST (websocket, confirmation_options_update)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	std::atomic<bool> added{ false };
	std::atomic<bool> deleted{ false };
	auto task = ([&added, &deleted, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		// Subscribe initially with empty options, everything will be filtered
		client.send_message (R"json({"action": "subscribe", "topic": "confirmation", "ack": true, "options": {}})json");
		client.await_ack ();
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::confirmation));
		// Now update filter with an account and wait for a response
		std::string add_message = boost::str (boost::format (R"json({"action": "update", "topic": "confirmation", "ack": true, "options": {"accounts_add": ["%1%"]}})json") % nano::dev::genesis_key.pub.to_account ());
		client.send_message (add_message);
		client.await_ack ();
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::confirmation));
		added = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::confirmation));
		EXPECT_TRUE (client.get_response ());
		// Update the filter again, removing the account
		std::string delete_message = boost::str (boost::format (R"json({"action": "update", "topic": "confirmation", "ack": true, "options": {"accounts_del": ["%1%"]}})json") % nano::dev::genesis_key.pub.to_account ());
		client.send_message (delete_message);
		client.await_ack ();
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::confirmation));
		deleted = true;
		EXPECT_FALSE (client.get_response (1s));
	});
	auto future = std::async (std::launch::async, task);

	// Wait for update acknowledgement
	ASSERT_TIMELY_EQ (5s, added.load (), true);

	// Confirm a block
	(void)node1->wallets.insert_adhoc (node1->wallets.first_wallet_id (), nano::dev::genesis_key.prv);
	nano::keypair key;
	nano::state_block_builder builder;
	auto previous (node1->latest (nano::dev::genesis_key.pub));
	auto send = builder
				.account (nano::dev::genesis_key.pub)
				.previous (previous)
				.representative (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - nano::Gxrb_ratio)
				.link (key.pub)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (previous))
				.build ();

	node1->process_active (send);

	// Wait for delete acknowledgement
	ASSERT_TIMELY (5s, deleted);

	// Confirm another block
	previous = send->hash ();
	auto send2 = builder
				 .make_block ()
				 .account (nano::dev::genesis_key.pub)
				 .previous (previous)
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - 2 * nano::Gxrb_ratio)
				 .link (key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (previous))
				 .build ();

	node1->process_active (send2);

	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);
}

// Subscribes to votes, sends a block and awaits websocket notification of a vote arrival
TEST (websocket, vote)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	std::atomic<bool> ack_ready{ false };
	auto task = ([&ack_ready, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "vote", "ack": true})json");
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::vote));
		return client.get_response ();
	});
	auto future = std::async (std::launch::async, task);

	ASSERT_TIMELY (5s, ack_ready);

	// Quick-confirm a block
	nano::keypair key;
	nano::state_block_builder builder;
	(void)node1->wallets.insert_adhoc (node1->wallets.first_wallet_id (), nano::dev::genesis_key.prv);
	nano::block_hash previous (node1->latest (nano::dev::genesis_key.pub));
	auto send = builder
				.account (nano::dev::genesis_key.pub)
				.previous (previous)
				.representative (nano::dev::genesis_key.pub)
				.balance (nano::dev::constants.genesis_amount - (node1->quorum ().quorum_delta.number () + 1))
				.link (key.pub)
				.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				.work (*system.work.generate (previous))
				.build ();

	node1->process_active (send);

	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);

	auto response = future.get ();
	ASSERT_TRUE (response);
	boost::property_tree::ptree event;
	std::stringstream stream;
	stream << response;
	boost::property_tree::read_json (stream, event);
	ASSERT_EQ (event.get<std::string> ("topic"), "vote");
}

// Tests vote subscription options - vote type
TEST (websocket, vote_options_type)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	std::atomic<bool> ack_ready{ false };
	auto task = ([&ack_ready, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "vote", "ack": true, "options": {"include_replays": true, "include_indeterminate": false}})json");
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::vote));
		return client.get_response ();
	});
	auto future = std::async (std::launch::async, task);

	ASSERT_TIMELY (5s, ack_ready);

	// Custom made votes for simplicity
	auto vote = nano::test::make_vote (nano::dev::genesis_key, { nano::dev::genesis }, 0, 0);
	nano::websocket::message_builder builder;
	auto msg (builder.vote_received (vote, nano::vote_code::replay));
	node1->websocket.server->broadcast (msg);

	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);

	auto response = future.get ();
	ASSERT_TRUE (response);
	boost::property_tree::ptree event;
	std::stringstream stream;
	stream << response;
	boost::property_tree::read_json (stream, event);
	auto message_contents = event.get_child ("message");
	ASSERT_EQ (1, message_contents.count ("type"));
	ASSERT_EQ ("replay", message_contents.get<std::string> ("type"));
}

// Tests vote subscription options - list of representatives
TEST (websocket, vote_options_representatives)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	std::atomic<bool> ack_ready{ false };
	auto task1 = ([&ack_ready, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		std::string message = boost::str (boost::format (R"json({"action": "subscribe", "topic": "vote", "ack": true, "options": {"representatives": ["%1%"]}})json") % nano::dev::genesis_key.pub.to_account ());
		client.send_message (message);
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::vote));
		auto response = client.get_response ();
		EXPECT_TRUE (response);
		boost::property_tree::ptree event;
		std::stringstream stream;
		stream << response;
		boost::property_tree::read_json (stream, event);
		EXPECT_EQ (event.get<std::string> ("topic"), "vote");
	});
	auto future1 = std::async (std::launch::async, task1);

	ASSERT_TIMELY (5s, ack_ready);

	// Quick-confirm a block
	nano::keypair key;
	auto balance = nano::dev::constants.genesis_amount;
	(void)node1->wallets.insert_adhoc (node1->wallets.first_wallet_id (), nano::dev::genesis_key.prv);
	auto send_amount = node1->quorum ().quorum_delta.number () + 1;
	auto confirm_block = [&] () {
		nano::block_hash previous (node1->latest (nano::dev::genesis_key.pub));
		balance -= send_amount;
		nano::state_block_builder builder;
		auto send = builder
					.account (nano::dev::genesis_key.pub)
					.previous (previous)
					.representative (nano::dev::genesis_key.pub)
					.balance (balance)
					.link (key.pub)
					.sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
					.work (*system.work.generate (previous))
					.build ();
		node1->process_active (send);
	};
	confirm_block ();

	ASSERT_TIMELY_EQ (5s, future1.wait_for (0s), std::future_status::ready);

	ack_ready = false;
	auto task2 = ([&ack_ready, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "vote", "ack": true, "options": {"representatives": ["xrb_invalid"]}})json");
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::vote));
		auto response = client.get_response ();
		// A list of invalid representatives is the same as no filter
		EXPECT_TRUE (response);
	});
	auto future2 = std::async (std::launch::async, task2);

	// Wait for the subscription to be acknowledged
	ASSERT_TIMELY (5s, ack_ready);

	// Confirm another block
	confirm_block ();

	ASSERT_TIMELY_EQ (5s, future2.wait_for (0s), std::future_status::ready);
}

// Test client subscribing to notifications for work generation
// disabled, because distributed work generation was temporarily removed
TEST (websocket, DISABLED_work)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	ASSERT_EQ (0, node1->websocket.server->subscriber_count (nano::websocket::topic::work));

	// Subscribe to work and wait for response asynchronously
	std::atomic<bool> ack_ready{ false };
	auto task = ([&ack_ready, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "work", "ack": true})json");
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::work));
		return client.get_response ();
	});
	auto future = std::async (std::launch::async, task);

	// Wait for acknowledge
	ASSERT_TIMELY (5s, ack_ready);
	ASSERT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::work));

	// Generate work
	nano::block_hash hash{ 1 };
	auto work (node1->work_generate_blocking (hash));
	ASSERT_TRUE (work.has_value ());

	// Wait for the work notification
	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);

	// Check the work notification message
	auto response = future.get ();
	ASSERT_TRUE (response);
	std::stringstream stream;
	stream << response;
	boost::property_tree::ptree event;
	boost::property_tree::read_json (stream, event);
	ASSERT_EQ (event.get<std::string> ("topic"), "work");

	auto & contents = event.get_child ("message");
	ASSERT_EQ (contents.get<std::string> ("success"), "true");
	ASSERT_LT (contents.get<unsigned> ("duration"), 10000U);

	ASSERT_EQ (1, contents.count ("request"));
	auto & request = contents.get_child ("request");
	ASSERT_EQ (request.get<std::string> ("version"), nano::to_string (nano::work_version::work_1));
	ASSERT_EQ (request.get<std::string> ("hash"), hash.to_string ());
	ASSERT_EQ (request.get<std::string> ("difficulty"), nano::to_string_hex (node1->default_difficulty (nano::work_version::work_1)));
	ASSERT_EQ (request.get<double> ("multiplier"), 1.0);

	ASSERT_EQ (1, contents.count ("result"));
	auto & result = contents.get_child ("result");
	uint64_t result_difficulty;
	nano::from_string_hex (result.get<std::string> ("difficulty"), result_difficulty);
	ASSERT_GE (result_difficulty, node1->default_difficulty (nano::work_version::work_1));
	ASSERT_NEAR (result.get<double> ("multiplier"), nano::difficulty::to_multiplier (result_difficulty, node1->default_difficulty (nano::work_version::work_1)), 1e-6);
	ASSERT_EQ (result.get<std::string> ("work"), nano::to_string_hex (work.value ()));

	ASSERT_EQ (1, contents.count ("bad_peers"));
	auto & bad_peers = contents.get_child ("bad_peers");
	ASSERT_TRUE (bad_peers.empty ());

	ASSERT_EQ (contents.get<std::string> ("reason"), "");
}

// Test client subscribing to notifications for bootstrap
TEST (websocket, bootstrap)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	ASSERT_EQ (0, node1->websocket.server->subscriber_count (nano::websocket::topic::bootstrap));

	// Subscribe to bootstrap and wait for response asynchronously
	std::atomic<bool> ack_ready{ false };
	auto task = ([&ack_ready, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "bootstrap", "ack": true})json");
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::bootstrap));
		return client.get_response ();
	});
	auto future = std::async (std::launch::async, task);

	// Wait for acknowledge
	ASSERT_TIMELY (5s, ack_ready);

	// Start bootstrap attempt
	node1->bootstrap_initiator.bootstrap (true, "123abc");
	ASSERT_TIMELY_EQ (5s, false, node1->bootstrap_initiator.has_legacy_attempt ());

	// Wait for the bootstrap notification
	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);

	// Check the bootstrap notification message
	auto response = future.get ();
	ASSERT_TRUE (response);
	std::stringstream stream;
	stream << response;
	boost::property_tree::ptree event;
	boost::property_tree::read_json (stream, event);
	ASSERT_EQ (event.get<std::string> ("topic"), "bootstrap");

	auto & contents = event.get_child ("message");
	ASSERT_EQ (contents.get<std::string> ("reason"), "started");
	ASSERT_EQ (contents.get<std::string> ("id"), "123abc");
	ASSERT_EQ (contents.get<std::string> ("mode"), "legacy");

	// Wait for bootstrap finish
	ASSERT_TIMELY (5s, !node1->bootstrap_initiator.in_progress ());
}

// Disabled, so that bootstrapping can be refactored. Rewrite in a simpler way!
TEST (websocket, DISABLED_bootstrap_exited)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	// Start bootstrap, exit after subscription
	std::atomic<bool> bootstrap_started{ false };
	nano::test::counted_completion subscribed_completion (1);
	std::thread bootstrap_thread ([node1, &system, &bootstrap_started, &subscribed_completion] () {
		bool has_legacy = false;
		while (has_legacy == false)
		{
			std::this_thread::sleep_for (50ms);
			node1->bootstrap_initiator.bootstrap (true, "123abc");
			has_legacy = node1->bootstrap_initiator.has_legacy_attempt ();
		}
		// TODO: bootstrap attempt gets dropped. That's why the test fails
		bootstrap_started = true;
		EXPECT_FALSE (subscribed_completion.await_count_for (5s));
	});

	// Wait for bootstrap start
	ASSERT_TIMELY (5s, bootstrap_started);

	// Subscribe to bootstrap and wait for response asynchronously
	std::atomic<bool> ack_ready{ false };
	auto task = ([&ack_ready, config, &node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "bootstrap", "ack": true})json");
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::bootstrap));
		return client.get_response ();
	});
	auto future = std::async (std::launch::async, task);

	// Wait for acknowledge
	ASSERT_TIMELY (5s, ack_ready);

	// Wait for the bootstrap notification
	subscribed_completion.increment ();
	bootstrap_thread.join ();
	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);

	// Check the bootstrap notification message
	auto response = future.get ();
	ASSERT_TRUE (response);
	std::stringstream stream;
	stream << response;
	boost::property_tree::ptree event;
	boost::property_tree::read_json (stream, event);
	ASSERT_EQ (event.get<std::string> ("topic"), "bootstrap");

	auto & contents = event.get_child ("message");
	ASSERT_EQ (contents.get<std::string> ("reason"), "exited");
	ASSERT_EQ (contents.get<std::string> ("id"), "123abc");
	ASSERT_EQ (contents.get<std::string> ("mode"), "legacy");
	ASSERT_EQ (contents.get<unsigned> ("total_blocks"), 0U);
	ASSERT_LT (contents.get<unsigned> ("duration"), 15000U);
}

// Tests sending keepalive
TEST (websocket, ws_keepalive)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	auto task = ([&node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "ping"})json");
		client.await_ack ();
	});
	auto future = std::async (std::launch::async, task);

	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);
}

// Tests sending telemetry
TEST (websocket, telemetry)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	nano::node_flags node_flags;
	auto node1 (system.add_node (config, node_flags));
	config.peering_port = system.get_available_port ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node2 (system.add_node (config, node_flags));

	nano::test::wait_peer_connections (system);

	std::atomic<bool> done{ false };
	auto task = ([config = node1->config, &node1, &done] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "telemetry", "ack": true})json");
		client.await_ack ();
		done = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::telemetry));
		return client.get_response ();
	});

	auto future = std::async (std::launch::async, task);

	ASSERT_TIMELY (10s, done);

	auto remote = node1->find_endpoint_for_node_id (node2->get_node_id ());
	ASSERT_TRUE (remote.has_value ());
	ASSERT_TIMELY (5s, node1->telemetry->get_telemetry (remote.value ()));

	ASSERT_TIMELY_EQ (10s, future.wait_for (0s), std::future_status::ready);

	// Check the telemetry notification message
	auto response = future.get ();

	std::stringstream stream;
	stream << response;
	boost::property_tree::ptree event;
	boost::property_tree::read_json (stream, event);
	ASSERT_EQ (event.get<std::string> ("topic"), "telemetry");

	auto & contents = event.get_child ("message");
	nano::jsonconfig telemetry_contents (contents);
	nano::telemetry_data telemetry_data;
	telemetry_data.deserialize_json (telemetry_contents, false);

	ASSERT_TRUE (nano::test::compare_telemetry (telemetry_data, *node2));

	ASSERT_EQ (contents.get<std::string> ("address"), remote.value ().address ().to_string ());
	ASSERT_EQ (contents.get<uint16_t> ("port"), remote.value ().port ());

	// Other node should have no subscribers
	EXPECT_EQ (0, node2->websocket.server->subscriber_count (nano::websocket::topic::telemetry));
}

TEST (websocket, new_unconfirmed_block)
{
	nano::test::system system;
	nano::node_config config = system.default_config ();
	config.websocket_config.enabled = true;
	config.websocket_config.port = system.get_available_port ();
	auto node1 (system.add_node (config));

	std::atomic<bool> ack_ready{ false };
	auto task = ([&ack_ready, config, node1] () {
		fake_websocket_client client (node1->websocket.server->listening_port ());
		client.send_message (R"json({"action": "subscribe", "topic": "new_unconfirmed_block", "ack": true})json");
		client.await_ack ();
		ack_ready = true;
		EXPECT_EQ (1, node1->websocket.server->subscriber_count (nano::websocket::topic::new_unconfirmed_block));
		return client.get_response ();
	});
	auto future = std::async (std::launch::async, task);

	ASSERT_TIMELY (5s, ack_ready);

	nano::state_block_builder builder;
	// Process a new block
	auto send1 = builder
				 .account (nano::dev::genesis_key.pub)
				 .previous (nano::dev::genesis->hash ())
				 .representative (nano::dev::genesis_key.pub)
				 .balance (nano::dev::constants.genesis_amount - 1)
				 .link (nano::dev::genesis_key.pub)
				 .sign (nano::dev::genesis_key.prv, nano::dev::genesis_key.pub)
				 .work (*system.work.generate (nano::dev::genesis->hash ()))
				 .build ();

	ASSERT_EQ (nano::block_status::progress, node1->process_local (send1).value ());

	ASSERT_TIMELY_EQ (5s, future.wait_for (0s), std::future_status::ready);

	// Check the response
	boost::optional<std::string> response = future.get ();
	ASSERT_TRUE (response);
	std::stringstream stream;
	stream << response;
	boost::property_tree::ptree event;
	boost::property_tree::read_json (stream, event);
	ASSERT_EQ (event.get<std::string> ("topic"), "new_unconfirmed_block");

	auto message_contents = event.get_child ("message");
	ASSERT_EQ ("state", message_contents.get<std::string> ("type"));
	ASSERT_EQ ("send", message_contents.get<std::string> ("subtype"));
}
