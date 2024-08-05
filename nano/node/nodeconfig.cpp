#include "nano/lib/rsnano.hpp"
#include "nano/node/active_elections.hpp"

#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/lib/config.hpp>
#include <nano/lib/jsonconfig.hpp>
#include <nano/lib/rpcconfig.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/tomlconfig.hpp>
#include <nano/node/nodeconfig.hpp>
#include <nano/node/transport/transport.hpp>

#include <boost/format.hpp>

namespace
{
char const * preconfigured_peers_key = "preconfigured_peers";
char const * signature_checker_threads_key = "signature_checker_threads";
char const * pow_sleep_interval_key = "pow_sleep_interval";
}

rsnano::NodeConfigDto to_node_config_dto (nano::node_config const & config)
{
	rsnano::NodeConfigDto dto;
	dto.optimistic_scheduler = config.optimistic_scheduler.into_dto ();
	dto.hinted_scheduler = config.hinted_scheduler.into_dto ();
	dto.priority_bucket = config.priority_bucket.into_dto ();
	dto.peering_port = config.peering_port.value_or (0);
	dto.peering_port_defined = config.peering_port.has_value ();
	dto.bootstrap_fraction_numerator = config.bootstrap_fraction_numerator;
	dto.bootstrap_ascending = config.bootstrap_ascending.to_dto ();
	dto.bootstrap_server = config.bootstrap_server.to_dto ();
	std::copy (std::begin (config.receive_minimum.bytes), std::end (config.receive_minimum.bytes), std::begin (dto.receive_minimum));
	std::copy (std::begin (config.online_weight_minimum.bytes), std::end (config.online_weight_minimum.bytes), std::begin (dto.online_weight_minimum));
	std::copy (std::begin (config.representative_vote_weight_minimum.bytes), std::end (config.representative_vote_weight_minimum.bytes), std::begin (dto.representative_vote_weight_minimum));
	dto.password_fanout = config.password_fanout;
	dto.io_threads = config.io_threads;
	dto.network_threads = config.network_threads;
	dto.work_threads = config.work_threads;
	dto.background_threads = config.background_threads;
	dto.signature_checker_threads = config.signature_checker_threads;
	dto.enable_voting = config.enable_voting;
	dto.bootstrap_connections = config.bootstrap_connections;
	dto.bootstrap_connections_max = config.bootstrap_connections_max;
	dto.bootstrap_initiator_threads = config.bootstrap_initiator_threads;
	dto.bootstrap_serving_threads = config.bootstrap_serving_threads;
	dto.bootstrap_frontier_request_count = config.bootstrap_frontier_request_count;
	dto.block_processor_batch_max_time_ms = config.block_processor_batch_max_time.count ();
	dto.allow_local_peers = config.allow_local_peers;
	std::copy (std::begin (config.vote_minimum.bytes), std::end (config.vote_minimum.bytes), std::begin (dto.vote_minimum));
	dto.vote_generator_delay_ms = config.vote_generator_delay.count ();
	dto.vote_generator_threshold = config.vote_generator_threshold;
	dto.unchecked_cutoff_time_s = config.unchecked_cutoff_time.count ();
	dto.tcp_io_timeout_s = config.tcp_io_timeout.count ();
	dto.pow_sleep_interval_ns = config.pow_sleep_interval.count ();
	std::copy (config.external_address.begin (), config.external_address.end (), std::begin (dto.external_address));
	dto.external_address_len = config.external_address.length ();
	dto.external_port = config.external_port;
	dto.tcp_incoming_connections_max = config.tcp_incoming_connections_max;
	dto.use_memory_pools = config.use_memory_pools;
	dto.bandwidth_limit = config.bandwidth_limit;
	dto.bandwidth_limit_burst_ratio = config.bandwidth_limit_burst_ratio;
	dto.bootstrap_bandwidth_limit = config.bootstrap_bandwidth_limit;
	dto.bootstrap_bandwidth_burst_ratio = config.bootstrap_bandwidth_burst_ratio;
	dto.confirming_set_batch_time_ms = config.confirming_set_batch_time.count ();
	dto.backup_before_upgrade = config.backup_before_upgrade;
	dto.max_work_generate_multiplier = config.max_work_generate_multiplier;
	dto.frontiers_confirmation = static_cast<uint8_t> (config.frontiers_confirmation);
	dto.max_queued_requests = config.max_queued_requests;
	dto.request_aggregator_threads = config.request_aggregator_threads;
	dto.max_unchecked_blocks = config.max_unchecked_blocks;
	std::copy (std::begin (config.rep_crawler_weight_minimum.bytes), std::end (config.rep_crawler_weight_minimum.bytes), std::begin (dto.rep_crawler_weight_minimum));
	dto.work_peers_count = config.work_peers.size ();
	dto.backlog_scan_batch_size = config.backlog_scan_batch_size;
	dto.backlog_scan_frequency = config.backlog_scan_frequency;
	for (auto i = 0; i < config.work_peers.size (); i++)
	{
		std::copy (config.work_peers[i].first.begin (), config.work_peers[i].first.end (), std::begin (dto.work_peers[i].address));
		dto.work_peers[i].address_len = config.work_peers[i].first.size ();
		dto.work_peers[i].port = config.work_peers[i].second;
	}
	dto.secondary_work_peers_count = config.secondary_work_peers.size ();
	for (auto i = 0; i < config.secondary_work_peers.size (); i++)
	{
		std::copy (config.secondary_work_peers[i].first.begin (), config.secondary_work_peers[i].first.end (), std::begin (dto.secondary_work_peers[i].address));
		dto.secondary_work_peers[i].address_len = config.secondary_work_peers[i].first.size ();
		dto.secondary_work_peers[i].port = config.secondary_work_peers[i].second;
	}
	dto.preconfigured_peers_count = config.preconfigured_peers.size ();
	for (auto i = 0; i < config.preconfigured_peers.size (); i++)
	{
		std::copy (config.preconfigured_peers[i].begin (), config.preconfigured_peers[i].end (), std::begin (dto.preconfigured_peers[i].address));
		dto.preconfigured_peers[i].address_len = config.preconfigured_peers[i].size ();
	}
	for (auto i = 0; i < config.preconfigured_representatives.size (); i++)
	{
		std::copy (std::begin (config.preconfigured_representatives[i].bytes), std::end (config.preconfigured_representatives[i].bytes), std::begin (dto.preconfigured_representatives[i]));
		dto.preconfigured_representatives_count = config.preconfigured_representatives.size ();
	}
	dto.preconfigured_representatives_count = config.preconfigured_representatives.size ();
	dto.max_pruning_age_s = config.max_pruning_age.count ();
	dto.max_pruning_depth = config.max_pruning_depth;
	std::copy (config.callback_address.begin (), config.callback_address.end (), std::begin (dto.callback_address));
	dto.callback_address_len = config.callback_address.size ();
	std::copy (config.callback_target.begin (), config.callback_target.end (), std::begin (dto.callback_target));
	dto.callback_target_len = config.callback_target.size ();
	dto.callback_port = config.callback_port;
	dto.websocket_config = config.websocket_config.to_dto ();
	dto.ipc_config = config.ipc_config.to_dto ();
	dto.diagnostics_config = config.diagnostics_config.to_dto ();
	dto.stat_config = config.stats_config.to_dto ();
	dto.lmdb_config = config.lmdb_config.to_dto ();
	dto.vote_cache = config.vote_cache.to_dto ();
	dto.rep_crawler_query_timeout_ms = config.rep_crawler.query_timeout.count ();
	dto.block_processor = config.block_processor.to_dto ();
	dto.active_elections = config.active_elections.into_dto ();
	dto.vote_processor = config.vote_processor.to_dto ();
	dto.tcp = config.tcp.to_dto ();
	dto.request_aggregator = config.request_aggregator.into_dto ();
	dto.message_processor = config.message_processor.into_dto ();
	dto.priority_scheduler_enabled = config.priority_scheduler_enabled;
	dto.local_block_broadcaster = config.local_block_broadcaster.into_dto ();
	dto.confirming_set = config.confirming_set.into_dto ();
	dto.monitor = config.monitor.into_dto ();
	return dto;
}

nano::node_config::node_config (nano::network_params & network_params) :
	node_config (std::nullopt, network_params)
{
}

nano::node_config::node_config (const std::optional<uint16_t> & peering_port_a, nano::network_params & network_params) :
	network_params{ network_params },
	websocket_config{ network_params.network },
	ipc_config (network_params.network),
	rep_crawler{ std::chrono::milliseconds (0) }
{
	rsnano::NodeConfigDto dto;
	auto network_params_dto{ network_params.to_dto () };
	rsnano::rsn_node_config_create (&dto, peering_port_a.value_or (0), peering_port_a.has_value (), &network_params_dto);
	load_dto (dto);
}

rsnano::NodeConfigDto nano::node_config::to_dto () const
{
	return to_node_config_dto (*this);
}

void nano::node_config::load_dto (rsnano::NodeConfigDto & dto)
{
	if (dto.peering_port_defined)
	{
		peering_port = dto.peering_port;
	}
	else
	{
		peering_port = std::nullopt;
	}
	optimistic_scheduler.load_dto (dto.optimistic_scheduler);
	hinted_scheduler.load_dto (dto.hinted_scheduler);
	priority_bucket = nano::priority_bucket_config{ dto.priority_bucket };
	bootstrap_fraction_numerator = dto.bootstrap_fraction_numerator;
	bootstrap_ascending.load_dto (dto.bootstrap_ascending);
	bootstrap_server.load_dto (dto.bootstrap_server);
	std::copy (std::begin (dto.receive_minimum), std::end (dto.receive_minimum), std::begin (receive_minimum.bytes));
	std::copy (std::begin (dto.online_weight_minimum), std::end (dto.online_weight_minimum), std::begin (online_weight_minimum.bytes));
	std::copy (std::begin (dto.representative_vote_weight_minimum), std::end (dto.representative_vote_weight_minimum), std::begin (representative_vote_weight_minimum.bytes));
	password_fanout = dto.password_fanout;
	io_threads = dto.io_threads;
	network_threads = dto.network_threads;
	work_threads = dto.work_threads;
	background_threads = dto.background_threads;
	signature_checker_threads = dto.signature_checker_threads;
	enable_voting = dto.enable_voting;
	bootstrap_connections = dto.bootstrap_connections;
	bootstrap_connections_max = dto.bootstrap_connections_max;
	bootstrap_initiator_threads = dto.bootstrap_initiator_threads;
	bootstrap_serving_threads = dto.bootstrap_serving_threads;
	bootstrap_frontier_request_count = dto.bootstrap_frontier_request_count;
	block_processor_batch_max_time = std::chrono::milliseconds (dto.block_processor_batch_max_time_ms);
	allow_local_peers = dto.allow_local_peers;
	std::copy (std::begin (dto.vote_minimum), std::end (dto.vote_minimum), std::begin (vote_minimum.bytes));
	vote_generator_delay = std::chrono::milliseconds (dto.vote_generator_delay_ms);
	vote_generator_threshold = dto.vote_generator_threshold;
	unchecked_cutoff_time = std::chrono::seconds (dto.unchecked_cutoff_time_s);
	tcp_io_timeout = std::chrono::seconds (dto.tcp_io_timeout_s);
	pow_sleep_interval = std::chrono::nanoseconds (dto.pow_sleep_interval_ns);
	external_address = std::string (reinterpret_cast<const char *> (dto.external_address), dto.external_address_len);
	external_port = dto.external_port;
	tcp_incoming_connections_max = dto.tcp_incoming_connections_max;
	use_memory_pools = dto.use_memory_pools;
	bandwidth_limit = dto.bandwidth_limit;
	bandwidth_limit_burst_ratio = dto.bandwidth_limit_burst_ratio;
	bootstrap_bandwidth_limit = dto.bootstrap_bandwidth_limit;
	bootstrap_bandwidth_burst_ratio = dto.bootstrap_bandwidth_burst_ratio;
	confirming_set_batch_time = std::chrono::milliseconds (dto.confirming_set_batch_time_ms);
	backup_before_upgrade = dto.backup_before_upgrade;
	max_work_generate_multiplier = dto.max_work_generate_multiplier;
	frontiers_confirmation = static_cast<nano::frontiers_confirmation_mode> (dto.frontiers_confirmation);
	max_queued_requests = dto.max_queued_requests;
	request_aggregator_threads = dto.request_aggregator_threads;
	max_unchecked_blocks = dto.max_unchecked_blocks;
	std::copy (std::begin (dto.rep_crawler_weight_minimum), std::end (dto.rep_crawler_weight_minimum), std::begin (rep_crawler_weight_minimum.bytes));
	work_peers.clear ();
	for (auto i = 0; i < dto.work_peers_count; i++)
	{
		std::string address (reinterpret_cast<const char *> (dto.work_peers[i].address), dto.work_peers[i].address_len);
		work_peers.push_back (std::make_pair (address, dto.work_peers[i].port));
	}
	secondary_work_peers.clear ();
	for (auto i = 0; i < dto.secondary_work_peers_count; i++)
	{
		std::string address (reinterpret_cast<const char *> (dto.secondary_work_peers[i].address), dto.secondary_work_peers[i].address_len);
		secondary_work_peers.push_back (std::make_pair (address, dto.secondary_work_peers[i].port));
	}
	preconfigured_peers.clear ();
	for (auto i = 0; i < dto.preconfigured_peers_count; i++)
	{
		std::string address (reinterpret_cast<const char *> (dto.preconfigured_peers[i].address), dto.preconfigured_peers[i].address_len);
		preconfigured_peers.push_back (address);
	}
	preconfigured_representatives.clear ();
	for (auto i = 0; i < dto.preconfigured_representatives_count; i++)
	{
		nano::account a;
		std::copy (std::begin (dto.preconfigured_representatives[i]), std::end (dto.preconfigured_representatives[i]), std::begin (a.bytes));
		preconfigured_representatives.push_back (a);
	}
	max_pruning_age = std::chrono::seconds (dto.max_pruning_age_s);
	max_pruning_depth = dto.max_pruning_depth;
	callback_address = std::string (reinterpret_cast<const char *> (dto.callback_address), dto.callback_address_len);
	callback_target = std::string (reinterpret_cast<const char *> (dto.callback_target), dto.callback_target_len);
	callback_port = dto.callback_port;
	websocket_config.load_dto (dto.websocket_config);
	ipc_config.load_dto (dto.ipc_config);
	diagnostics_config.load_dto (dto.diagnostics_config);
	stats_config.load_dto (dto.stat_config);
	lmdb_config.load_dto (dto.lmdb_config);
	backlog_scan_batch_size = dto.backlog_scan_batch_size;
	backlog_scan_frequency = dto.backlog_scan_frequency;
	vote_cache = nano::vote_cache_config{ dto.vote_cache };
	rep_crawler.query_timeout = std::chrono::milliseconds (dto.rep_crawler_query_timeout_ms);
	block_processor = nano::block_processor_config{ dto.block_processor };
	active_elections = nano::active_elections_config{ dto.active_elections };
	vote_processor = nano::vote_processor_config{ dto.vote_processor };
	tcp = nano::transport::tcp_config{ dto.tcp };
	request_aggregator = nano::request_aggregator_config{ dto.request_aggregator };
	message_processor = nano::message_processor_config{ dto.message_processor };
	priority_scheduler_enabled = dto.priority_scheduler_enabled;
	local_block_broadcaster = nano::local_block_broadcaster_config{ dto.local_block_broadcaster };
	confirming_set = nano::confirming_set_config{ dto.confirming_set };
	monitor = nano::monitor_config{ dto.monitor };
}

nano::error nano::node_config::deserialize_toml (nano::tomlconfig & toml)
{
	try
	{
		if (toml.has_key ("httpcallback"))
		{
			auto callback_l (toml.get_required_child ("httpcallback"));
			callback_l.get<std::string> ("address", callback_address);
			callback_l.get<uint16_t> ("port", callback_port);
			callback_l.get<std::string> ("target", callback_target);
		}

		if (toml.has_key ("websocket"))
		{
			auto websocket_config_l (toml.get_required_child ("websocket"));
			websocket_config.deserialize_toml (websocket_config_l);
		}

		if (toml.has_key ("ipc"))
		{
			auto ipc_config_l (toml.get_required_child ("ipc"));
			ipc_config.deserialize_toml (ipc_config_l);
		}

		if (toml.has_key ("diagnostics"))
		{
			auto diagnostics_config_l (toml.get_required_child ("diagnostics"));
			diagnostics_config.deserialize_toml (diagnostics_config_l);
		}

		if (toml.has_key ("statistics"))
		{
			auto stats_config_l (toml.get_required_child ("statistics"));
			stats_config.deserialize_toml (stats_config_l);
		}

		if (toml.has_key ("optimistic_scheduler"))
		{
			auto config_l = toml.get_required_child ("optimistic_scheduler");
			optimistic_scheduler.deserialize (config_l);
		}

		if (toml.has_key ("priority_bucket"))
		{
			auto config_l = toml.get_required_child ("priority_bucket");
			priority_bucket.deserialize (config_l);
		}

		if (toml.has_key ("hinted_scheduler"))
		{
			auto config_l = toml.get_required_child ("hinted_scheduler");
			hinted_scheduler.deserialize (config_l);
		}

		if (toml.has_key ("bootstrap_ascending"))
		{
			auto config_l = toml.get_required_child ("bootstrap_ascending");
			bootstrap_ascending.deserialize (config_l);
		}

		if (toml.has_key ("bootstrap_server"))
		{
			auto config_l = toml.get_required_child ("bootstrap_server");
			bootstrap_server.deserialize (config_l);
		}

		if (toml.has_key ("vote_cache"))
		{
			auto config_l = toml.get_required_child ("vote_cache");
			vote_cache.deserialize (config_l);
		}

		if (toml.has_key ("rep_crawler"))
		{
			auto config_l = toml.get_required_child ("rep_crawler");
			rep_crawler.deserialize (config_l);
		}

		if (toml.has_key ("active_elections"))
		{
			auto config_l = toml.get_required_child ("active_elections");
			active_elections.deserialize (config_l);
		}

		if (toml.has_key ("block_processor"))
		{
			auto config_l = toml.get_required_child ("block_processor");
			block_processor.deserialize (config_l);
		}

		if (toml.has_key ("vote_processor"))
		{
			auto config_l = toml.get_required_child ("vote_processor");
			vote_processor.deserialize (config_l);
		}

		if (toml.has_key ("request_aggregator"))
		{
			auto config_l = toml.get_required_child ("request_aggregator");
			request_aggregator.deserialize (config_l);
		}

		if (toml.has_key ("message_processor"))
		{
			auto config_l = toml.get_required_child ("message_processor");
			message_processor.deserialize (config_l);
		}

		if (toml.has_key ("monitor"))
		{
			auto config_l = toml.get_required_child ("monitor");
			monitor.deserialize (config_l);
		}

		if (toml.has_key ("work_peers"))
		{
			work_peers.clear ();
			toml.array_entries_required<std::string> ("work_peers", [this] (std::string const & entry_a) {
				this->deserialize_address (entry_a, this->work_peers);
			});
		}

		if (toml.has_key (preconfigured_peers_key))
		{
			preconfigured_peers.clear ();
			toml.array_entries_required<std::string> (preconfigured_peers_key, [this] (std::string entry) {
				preconfigured_peers.push_back (entry);
			});
		}

		if (toml.has_key ("preconfigured_representatives"))
		{
			preconfigured_representatives.clear ();
			toml.array_entries_required<std::string> ("preconfigured_representatives", [this, &toml] (std::string entry) {
				nano::account representative{};
				if (representative.decode_account (entry))
				{
					toml.get_error ().set ("Invalid representative account: " + entry);
				}
				preconfigured_representatives.push_back (representative);
			});
		}

		if (preconfigured_representatives.empty ())
		{
			toml.get_error ().set ("At least one representative account must be set");
		}

		auto receive_minimum_l (receive_minimum.to_string_dec ());
		if (toml.has_key ("receive_minimum"))
		{
			receive_minimum_l = toml.get<std::string> ("receive_minimum");
		}
		if (receive_minimum.decode_dec (receive_minimum_l))
		{
			toml.get_error ().set ("receive_minimum contains an invalid decimal amount");
		}

		auto online_weight_minimum_l (online_weight_minimum.to_string_dec ());
		if (toml.has_key ("online_weight_minimum"))
		{
			online_weight_minimum_l = toml.get<std::string> ("online_weight_minimum");
		}
		if (online_weight_minimum.decode_dec (online_weight_minimum_l))
		{
			toml.get_error ().set ("online_weight_minimum contains an invalid decimal amount");
		}

		auto representative_vote_weight_minimum_l{ representative_vote_weight_minimum.to_string_dec () };
		if (toml.has_key ("representative_vote_weight_minimum"))
		{
			representative_vote_weight_minimum_l = toml.get<std::string> ("representative_vote_weight_minimum");
		}
		if (representative_vote_weight_minimum.decode_dec (representative_vote_weight_minimum_l))
		{
			toml.get_error ().set ("representative_vote_weight_minimum contains an invalid decimal amount");
		}

		auto vote_minimum_l (vote_minimum.to_string_dec ());
		if (toml.has_key ("vote_minimum"))
		{
			vote_minimum_l = toml.get<std::string> ("vote_minimum");
		}
		if (vote_minimum.decode_dec (vote_minimum_l))
		{
			toml.get_error ().set ("vote_minimum contains an invalid decimal amount");
		}

		auto delay_l = vote_generator_delay.count ();
		toml.get ("vote_generator_delay", delay_l);
		vote_generator_delay = std::chrono::milliseconds (delay_l);

		toml.get<unsigned> ("vote_generator_threshold", vote_generator_threshold);

		auto block_processor_batch_max_time_l = block_processor_batch_max_time.count ();
		toml.get ("block_processor_batch_max_time", block_processor_batch_max_time_l);
		block_processor_batch_max_time = std::chrono::milliseconds (block_processor_batch_max_time_l);

		auto unchecked_cutoff_time_l = static_cast<unsigned long> (unchecked_cutoff_time.count ());
		toml.get ("unchecked_cutoff_time", unchecked_cutoff_time_l);
		unchecked_cutoff_time = std::chrono::seconds (unchecked_cutoff_time_l);

		auto tcp_io_timeout_l = static_cast<unsigned long> (tcp_io_timeout.count ());
		toml.get ("tcp_io_timeout", tcp_io_timeout_l);
		tcp_io_timeout = std::chrono::seconds (tcp_io_timeout_l);

		if (toml.has_key ("peering_port"))
		{
			std::uint16_t peering_port_l{};
			toml.get_required<uint16_t> ("peering_port", peering_port_l);
			peering_port = peering_port_l;
		}

		toml.get<unsigned> ("bootstrap_fraction_numerator", bootstrap_fraction_numerator);
		toml.get<unsigned> ("password_fanout", password_fanout);
		toml.get<unsigned> ("io_threads", io_threads);
		toml.get<unsigned> ("work_threads", work_threads);
		toml.get<unsigned> ("network_threads", network_threads);
		toml.get<unsigned> ("background_threads", background_threads);
		toml.get<unsigned> ("bootstrap_connections", bootstrap_connections);
		toml.get<unsigned> ("bootstrap_connections_max", bootstrap_connections_max);
		toml.get<unsigned> ("bootstrap_initiator_threads", bootstrap_initiator_threads);
		toml.get<unsigned> ("bootstrap_serving_threads", bootstrap_serving_threads);
		toml.get<uint32_t> ("bootstrap_frontier_request_count", bootstrap_frontier_request_count);
		toml.get<bool> ("enable_voting", enable_voting);
		toml.get<bool> ("allow_local_peers", allow_local_peers);
		toml.get<unsigned> (signature_checker_threads_key, signature_checker_threads);

		if (toml.has_key ("lmdb"))
		{
			auto lmdb_config_l (toml.get_required_child ("lmdb"));
			lmdb_config.deserialize_toml (lmdb_config_l);
		}

		boost::asio::ip::address_v6 external_address_l;
		toml.get<boost::asio::ip::address_v6> ("external_address", external_address_l);
		external_address = external_address_l.to_string ();
		toml.get<uint16_t> ("external_port", external_port);
		toml.get<unsigned> ("tcp_incoming_connections_max", tcp_incoming_connections_max);

		auto pow_sleep_interval_l (pow_sleep_interval.count ());
		toml.get (pow_sleep_interval_key, pow_sleep_interval_l);
		pow_sleep_interval = std::chrono::nanoseconds (pow_sleep_interval_l);
		toml.get<bool> ("use_memory_pools", use_memory_pools);

		toml.get<std::size_t> ("bandwidth_limit", bandwidth_limit);
		toml.get<double> ("bandwidth_limit_burst_ratio", bandwidth_limit_burst_ratio);

		toml.get<std::size_t> ("bootstrap_bandwidth_limit", bootstrap_bandwidth_limit);
		toml.get<double> ("bootstrap_bandwidth_burst_ratio", bootstrap_bandwidth_burst_ratio);

		toml.get<bool> ("backup_before_upgrade", backup_before_upgrade);

		auto confirming_set_batch_time_l (confirming_set_batch_time.count ());
		toml.get ("confirming_set_batch_time", confirming_set_batch_time_l);
		confirming_set_batch_time = std::chrono::milliseconds (confirming_set_batch_time_l);

		toml.get<double> ("max_work_generate_multiplier", max_work_generate_multiplier);

		toml.get<uint32_t> ("max_queued_requests", max_queued_requests);
		toml.get<uint32_t> ("request_aggregator_threads", request_aggregator_threads);

		toml.get<unsigned> ("max_unchecked_blocks", max_unchecked_blocks);

		auto rep_crawler_weight_minimum_l (rep_crawler_weight_minimum.to_string_dec ());
		if (toml.has_key ("rep_crawler_weight_minimum"))
		{
			rep_crawler_weight_minimum_l = toml.get<std::string> ("rep_crawler_weight_minimum");
		}
		if (rep_crawler_weight_minimum.decode_dec (rep_crawler_weight_minimum_l))
		{
			toml.get_error ().set ("rep_crawler_weight_minimum contains an invalid decimal amount");
		}

		if (toml.has_key ("frontiers_confirmation"))
		{
			auto frontiers_confirmation_l (toml.get<std::string> ("frontiers_confirmation"));
			frontiers_confirmation = deserialize_frontiers_confirmation (frontiers_confirmation_l);
		}

		toml.get<unsigned> ("backlog_scan_batch_size", backlog_scan_batch_size);
		toml.get<unsigned> ("backlog_scan_frequency", backlog_scan_frequency);

		if (toml.has_key ("experimental"))
		{
			auto experimental_config_l (toml.get_required_child ("experimental"));
			if (experimental_config_l.has_key ("secondary_work_peers"))
			{
				secondary_work_peers.clear ();
				experimental_config_l.array_entries_required<std::string> ("secondary_work_peers", [this] (std::string const & entry_a) {
					this->deserialize_address (entry_a, this->secondary_work_peers);
				});
			}
			auto max_pruning_age_l (max_pruning_age.count ());
			experimental_config_l.get ("max_pruning_age", max_pruning_age_l);
			max_pruning_age = std::chrono::seconds (max_pruning_age_l);
			experimental_config_l.get<uint64_t> ("max_pruning_depth", max_pruning_depth);
		}

		if (password_fanout < 16 || password_fanout > 1024 * 1024)
		{
			toml.get_error ().set ("password_fanout must be a number between 16 and 1048576");
		}
		if (io_threads == 0)
		{
			toml.get_error ().set ("io_threads must be non-zero");
		}
		if (active_elections.size <= 250 && !network_params.network.is_dev_network ())
		{
			toml.get_error ().set ("active_elections.size must be greater than 250");
		}
		if (bandwidth_limit > std::numeric_limits<std::size_t>::max ())
		{
			toml.get_error ().set ("bandwidth_limit unbounded = 0, default = 10485760, max = 18446744073709551615");
		}
		if (vote_generator_threshold < 1 || vote_generator_threshold > 11)
		{
			toml.get_error ().set ("vote_generator_threshold must be a number between 1 and 11");
		}
		if (max_work_generate_multiplier < 1)
		{
			toml.get_error ().set ("max_work_generate_multiplier must be greater than or equal to 1");
		}
		if (frontiers_confirmation == nano::frontiers_confirmation_mode::invalid)
		{
			toml.get_error ().set ("frontiers_confirmation value is invalid (available: always, auto, disabled)");
		}
		if (block_processor_batch_max_time < network_params.node.process_confirmed_interval)
		{
			toml.get_error ().set ((boost::format ("block_processor_batch_max_time value must be equal or larger than %1%ms") % network_params.node.process_confirmed_interval.count ()).str ());
		}
		if (max_pruning_age < std::chrono::seconds (5 * 60) && !network_params.network.is_dev_network ())
		{
			toml.get_error ().set ("max_pruning_age must be greater than or equal to 5 minutes");
		}
		if (bootstrap_frontier_request_count < 1024)
		{
			toml.get_error ().set ("bootstrap_frontier_request_count must be greater than or equal to 1024");
		}
	}
	catch (std::runtime_error const & ex)
	{
		toml.get_error ().set (ex.what ());
	}

	return toml.get_error ();
}

nano::frontiers_confirmation_mode nano::node_config::deserialize_frontiers_confirmation (std::string const & string_a)
{
	if (string_a == "always")
	{
		return nano::frontiers_confirmation_mode::always;
	}
	else if (string_a == "auto")
	{
		return nano::frontiers_confirmation_mode::automatic;
	}
	else if (string_a == "disabled")
	{
		return nano::frontiers_confirmation_mode::disabled;
	}
	else
	{
		return nano::frontiers_confirmation_mode::invalid;
	}
}

void nano::node_config::deserialize_address (std::string const & entry_a, std::vector<std::pair<std::string, uint16_t>> & container_a) const
{
	auto port_position (entry_a.rfind (':'));
	bool result = (port_position == -1);
	if (!result)
	{
		auto port_str (entry_a.substr (port_position + 1));
		uint16_t port;
		result |= parse_port (port_str, port);
		if (!result)
		{
			auto address (entry_a.substr (0, port_position));
			container_a.emplace_back (address, port);
		}
	}
}

nano::account nano::node_config::random_representative () const
{
	debug_assert (!preconfigured_representatives.empty ());
	std::size_t index (nano::random_pool::generate_word32 (0, static_cast<uint32_t> (preconfigured_representatives.size () - 1)));
	auto result (preconfigured_representatives[index]);
	return result;
}

nano::node_flags::node_flags () :
	handle{ rsnano::rsn_node_flags_create () }
{
}

nano::node_flags::node_flags (nano::node_flags && other_a) :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::node_flags::node_flags (const nano::node_flags & other_a) :
	handle{ rsnano::rsn_node_flags_clone (other_a.handle) }
{
}

nano::node_flags::~node_flags ()
{
	if (handle)
		rsnano::rsn_node_flags_destroy (handle);
}

nano::node_flags & nano::node_flags::operator= (nano::node_flags const & other_a)
{
	if (handle != nullptr)
		rsnano::rsn_node_flags_destroy (handle);
	handle = rsnano::rsn_node_flags_clone (other_a.handle);
	return *this;
}

nano::node_flags & nano::node_flags::operator= (nano::node_flags && other_a)
{
	if (handle != nullptr)
		rsnano::rsn_node_flags_destroy (handle);
	handle = other_a.handle;
	other_a.handle = nullptr;
	return *this;
}

rsnano::NodeFlagsDto nano::node_flags::flags_dto () const
{
	rsnano::NodeFlagsDto dto;
	rsnano::rsn_node_flags_get (handle, &dto);
	return dto;
}

void nano::node_flags::set_flag (std::function<void (rsnano::NodeFlagsDto &)> const & callback)
{
	auto dto{ flags_dto () };
	callback (dto);
	rsnano::rsn_node_flags_set (handle, &dto);
}

std::vector<std::string> nano::node_flags::config_overrides () const
{
	std::array<rsnano::StringDto, 1000> overrides;
	auto count = rsnano::rsn_node_flags_config_overrides (handle, overrides.data (), overrides.size ());
	std::vector<std::string> result;
	result.reserve (count);
	for (auto i = 0; i < count; ++i)
	{
		result.push_back (rsnano::convert_dto_to_string (overrides[i]));
	}
	return result;
}

void nano::node_flags::set_config_overrides (const std::vector<std::string> & overrides)
{
	std::vector<int8_t const *> dtos;
	dtos.reserve (overrides.size ());
	for (const auto & s : overrides)
	{
		dtos.push_back (reinterpret_cast<const int8_t *> (s.data ()));
	}
	rsnano::rsn_node_flags_config_set_overrides (handle, dtos.data (), dtos.size ());
}

std::vector<std::string> nano::node_flags::rpc_config_overrides () const
{
	std::array<rsnano::StringDto, 1000> overrides;
	auto count = rsnano::rsn_node_flags_rpc_config_overrides (handle, overrides.data (), overrides.size ());
	std::vector<std::string> result;
	result.reserve (count);
	for (auto i = 0; i < count; ++i)
	{
		result.push_back (rsnano::convert_dto_to_string (overrides[i]));
	}
	return result;
}

void nano::node_flags::set_rpc_overrides (const std::vector<std::string> & overrides)
{
	std::vector<int8_t const *> dtos;
	dtos.reserve (overrides.size ());
	for (const auto & s : overrides)
	{
		dtos.push_back (reinterpret_cast<const int8_t *> (s.data ()));
	}
	rsnano::rsn_node_flags_rpc_config_set_overrides (handle, dtos.data (), dtos.size ());
}

bool nano::node_flags::disable_backup () const
{
	return flags_dto ().disable_backup;
}
void nano::node_flags::set_disable_activate_successors (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_activate_successors = value; });
}
void nano::node_flags::set_disable_backup (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_backup = value; });
}
bool nano::node_flags::disable_lazy_bootstrap () const
{
	return flags_dto ().disable_lazy_bootstrap;
}
void nano::node_flags::set_disable_lazy_bootstrap (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_lazy_bootstrap = value; });
}
bool nano::node_flags::disable_legacy_bootstrap () const
{
	return flags_dto ().disable_legacy_bootstrap;
}
void nano::node_flags::set_disable_legacy_bootstrap (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_legacy_bootstrap = value; });
}
bool nano::node_flags::disable_wallet_bootstrap () const
{
	return flags_dto ().disable_wallet_bootstrap;
}
void nano::node_flags::set_disable_wallet_bootstrap (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_wallet_bootstrap = value; });
}
bool nano::node_flags::disable_bootstrap_listener () const
{
	return flags_dto ().disable_bootstrap_listener;
}
void nano::node_flags::set_disable_bootstrap_listener (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_bootstrap_listener = value; });
}
bool nano::node_flags::disable_bootstrap_bulk_pull_server () const
{
	return flags_dto ().disable_bootstrap_bulk_pull_server;
}
void nano::node_flags::set_disable_bootstrap_bulk_pull_server (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_bootstrap_bulk_pull_server = value; });
}
bool nano::node_flags::disable_bootstrap_bulk_push_client () const
{
	return flags_dto ().disable_bootstrap_bulk_push_client;
}
void nano::node_flags::set_disable_bootstrap_bulk_push_client (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_bootstrap_bulk_push_client = value; });
}
bool nano::node_flags::disable_ongoing_bootstrap () const // For testing onl
{
	return flags_dto ().disable_ongoing_bootstrap;
}
void nano::node_flags::set_disable_ongoing_bootstrap (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_ongoing_bootstrap = value; });
}
bool nano::node_flags::disable_ascending_bootstrap () const
{
	return flags_dto ().disable_ascending_bootstrap;
}
void nano::node_flags::set_disable_ascending_bootstrap (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_ascending_bootstrap = value; });
}
bool nano::node_flags::disable_rep_crawler () const
{
	return flags_dto ().disable_rep_crawler;
}
void nano::node_flags::set_disable_rep_crawler (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_rep_crawler = value; });
}
bool nano::node_flags::disable_request_loop () const // For testing onl
{
	return flags_dto ().disable_request_loop;
}
void nano::node_flags::set_disable_request_loop (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_request_loop = value; });
}
bool nano::node_flags::disable_tcp_realtime () const
{
	return flags_dto ().disable_tcp_realtime;
}
void nano::node_flags::set_disable_tcp_realtime (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_tcp_realtime = value; });
}
bool nano::node_flags::disable_providing_telemetry_metrics () const
{
	return flags_dto ().disable_providing_telemetry_metrics;
}
void nano::node_flags::set_disable_providing_telemetry_metrics (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_providing_telemetry_metrics = value; });
}
bool nano::node_flags::disable_ongoing_telemetry_requests () const
{
	return flags_dto ().disable_ongoing_telemetry_requests;
}
void nano::node_flags::set_disable_ongoing_telemetry_requests (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_ongoing_telemetry_requests = value; });
}
bool nano::node_flags::disable_block_processor_unchecked_deletion () const
{
	return flags_dto ().disable_block_processor_unchecked_deletion;
}
void nano::node_flags::set_disable_block_processor_unchecked_deletion (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_block_processor_unchecked_deletion = value; });
}
bool nano::node_flags::disable_block_processor_republishing () const
{
	return flags_dto ().disable_block_processor_republishing;
}
void nano::node_flags::set_disable_block_processor_republishing (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_block_processor_republishing = value; });
}
bool nano::node_flags::allow_bootstrap_peers_duplicates () const
{
	return flags_dto ().allow_bootstrap_peers_duplicates;
}
void nano::node_flags::set_allow_bootstrap_peers_duplicates (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.allow_bootstrap_peers_duplicates = value; });
}
bool nano::node_flags::disable_max_peers_per_ip () const // For testing onl
{
	return flags_dto ().disable_max_peers_per_ip;
}
void nano::node_flags::set_disable_max_peers_per_ip (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_max_peers_per_ip = value; });
}
bool nano::node_flags::disable_max_peers_per_subnetwork () const // For testing onl
{
	return flags_dto ().disable_max_peers_per_subnetwork;
}
void nano::node_flags::set_disable_max_peers_per_subnetwork (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_max_peers_per_subnetwork = value; });
}
bool nano::node_flags::force_use_write_queue () const // For testing only
{
	return flags_dto ().force_use_write_queue;
}
void nano::node_flags::set_force_use_write_queue (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.force_use_write_queue = value; });
}
bool nano::node_flags::disable_search_pending () const // For testing only
{
	return flags_dto ().disable_search_pending;
}
void nano::node_flags::set_disable_search_pending (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_search_pending = value; });
}
bool nano::node_flags::enable_pruning () const
{
	return flags_dto ().enable_pruning;
}
void nano::node_flags::set_enable_pruning (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.enable_pruning = value; });
}
bool nano::node_flags::fast_bootstrap () const
{
	return flags_dto ().fast_bootstrap;
}
void nano::node_flags::set_fast_bootstrap (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.fast_bootstrap = value; });
}
bool nano::node_flags::read_only () const
{
	return flags_dto ().read_only;
}
void nano::node_flags::set_read_only (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.read_only = value; });
}
bool nano::node_flags::disable_connection_cleanup () const
{
	return flags_dto ().disable_connection_cleanup;
}
void nano::node_flags::set_disable_connection_cleanup (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.disable_connection_cleanup = value; });
}
nano::generate_cache_flags nano::node_flags::generate_cache () const
{
	return nano::generate_cache_flags{ rsnano::rsn_node_flags_generate_cache (handle) };
}
void nano::node_flags::set_generate_cache (nano::generate_cache_flags const & cache)
{
	rsnano::rsn_node_flags_generate_set_cache (handle, cache.handle);
}
bool nano::node_flags::inactive_node () const
{
	return flags_dto ().inactive_node;
}
void nano::node_flags::set_inactive_node (bool value)
{
	set_flag ([value] (rsnano::NodeFlagsDto & dto) { dto.inactive_node = value; });
}
std::size_t nano::node_flags::block_processor_batch_size () const
{
	return flags_dto ().block_processor_batch_size;
}
void nano::node_flags::set_block_processor_batch_size (std::size_t size)
{
	set_flag ([size] (rsnano::NodeFlagsDto & dto) { dto.block_processor_batch_size = size; });
}
std::size_t nano::node_flags::block_processor_full_size () const
{
	return flags_dto ().block_processor_full_size;
}
void nano::node_flags::set_block_processor_full_size (std::size_t size)
{
	set_flag ([size] (rsnano::NodeFlagsDto & dto) { dto.block_processor_full_size = size; });
}
std::size_t nano::node_flags::block_processor_verification_size () const
{
	return flags_dto ().block_processor_verification_size;
}
void nano::node_flags::set_block_processor_verification_size (std::size_t size)
{
	set_flag ([size] (rsnano::NodeFlagsDto & dto) { dto.block_processor_verification_size = size; });
}
std::size_t nano::node_flags::vote_processor_capacity () const
{
	return flags_dto ().vote_processor_capacity;
}
void nano::node_flags::set_vote_processor_capacity (std::size_t size)
{
	set_flag ([size] (rsnano::NodeFlagsDto & dto) { dto.vote_processor_capacity = size; });
}
std::size_t nano::node_flags::bootstrap_interval () const
{
	return flags_dto ().bootstrap_interval;
}
void nano::node_flags::set_bootstrap_interval (std::size_t size)
{
	set_flag ([size] (rsnano::NodeFlagsDto & dto) { dto.bootstrap_interval = size; });
}

nano::message_processor_config::message_processor_config (rsnano::MessageProcessorConfigDto const & dto) :
	threads{ dto.threads },
	max_queue{ dto.max_queue }
{
}

rsnano::MessageProcessorConfigDto nano::message_processor_config::into_dto () const
{
	return { threads, max_queue };
}

nano::error nano::message_processor_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("threads", threads);
	toml.get ("max_queue", max_queue);

	return toml.get_error ();
}

nano::local_block_broadcaster_config::local_block_broadcaster_config (rsnano::LocalBlockBroadcasterConfigDto const & dto) :
	max_size{ dto.max_size },
	rebroadcast_interval{ dto.rebroadcast_interval_s },
	max_rebroadcast_interval{ dto.rebroadcast_interval_s },
	broadcast_rate_limit{ dto.broadcast_rate_limit },
	broadcast_rate_burst_ratio{ dto.broadcast_rate_burst_ratio },
	cleanup_interval{ dto.cleanup_interval_s }
{
}

rsnano::LocalBlockBroadcasterConfigDto nano::local_block_broadcaster_config::into_dto () const
{
	return {
		max_size,
		static_cast<uint64_t> (rebroadcast_interval.count ()),
		static_cast<uint64_t> (max_rebroadcast_interval.count ()),
		broadcast_rate_limit,
		broadcast_rate_burst_ratio,
		static_cast<uint64_t> (cleanup_interval.count ())
	};
}

nano::confirming_set_config::confirming_set_config (rsnano::ConfirmingSetConfigDto const & dto) :
	max_blocks{ dto.max_blocks },
	max_queued_notifications{ dto.max_queued_notifications }
{
}

rsnano::ConfirmingSetConfigDto nano::confirming_set_config::into_dto () const
{
	return {
		max_blocks,
		max_queued_notifications
	};
}

nano::monitor_config::monitor_config (rsnano::MonitorConfigDto const & dto) :
	enabled{ dto.enabled },
	interval{ dto.interval_s }
{
}

rsnano::MonitorConfigDto nano::monitor_config::into_dto () const
{
	return {
		enabled,
		static_cast<uint64_t> (interval.count ())
	};
}

nano::error nano::monitor_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("enable", enabled);
	auto interval_l = interval.count ();
	toml.get ("interval", interval_l);
	interval = std::chrono::seconds{ interval_l };

	return toml.get_error ();
}

nano::priority_bucket_config::priority_bucket_config (rsnano::PriorityBucketConfigDto const & dto) :
	max_blocks{ dto.max_blocks },
	reserved_elections{ dto.reserved_elections },
	max_elections{ dto.max_elections }
{
}

rsnano::PriorityBucketConfigDto nano::priority_bucket_config::into_dto () const
{
	return {
		max_blocks,
		reserved_elections,
		max_elections
	};
}

nano::error nano::priority_bucket_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("max_blocks", max_blocks);
	toml.get ("reserved_elections", reserved_elections);
	toml.get ("max_elections", max_elections);

	return toml.get_error ();
}
