#include "nano/lib/rsnano.hpp"
#include "nano/lib/logging.hpp"
#include "nano/lib/rsnanoutils.hpp"
#include "nano/node/transport/tcp.hpp"

#include <nano/node/node.hpp>
#include <nano/node/repcrawler.hpp>

#include <boost/format.hpp>

#include <chrono>
#include <memory>
#include <stdexcept>

nano::representative::representative (nano::account account_a, std::shared_ptr<nano::transport::channel> const & channel_a) :
	handle{ rsnano::rsn_representative_create (account_a.bytes.data (), channel_a->handle) }
{
}

nano::representative::representative (rsnano::RepresentativeHandle * handle_a) :
	handle{ handle_a }
{
}

nano::representative::representative (representative const & other_a) :
	handle{ rsnano::rsn_representative_clone (other_a.handle) }
{
}

nano::representative::~representative ()
{
	rsnano::rsn_representative_destroy (handle);
}

nano::representative & nano::representative::operator= (nano::representative const & other_a)
{
	rsnano::rsn_representative_destroy (handle);
	handle = rsnano::rsn_representative_clone (other_a.handle);
	return *this;
}

nano::account nano::representative::get_account () const
{
	nano::account account;
	rsnano::rsn_representative_account (handle, account.bytes.data ());
	return account;
}

std::shared_ptr<nano::transport::channel> nano::representative::get_channel () const
{
	return nano::transport::channel_handle_to_channel (rsnano::rsn_representative_channel (handle));
}

void nano::representative::set_channel (std::shared_ptr<nano::transport::channel> new_channel)
{
	rsnano::rsn_representative_set_channel (handle, new_channel->handle);
}

std::chrono::system_clock::time_point nano::representative::get_last_request () const
{
	return rsnano::time_point_from_nanoseconds (rsnano::rsn_representative_last_request (handle));
}

void nano::representative::set_last_request (std::chrono::system_clock::time_point time_point)
{
	auto timepoint_ns = std::chrono::duration_cast<std::chrono::nanoseconds> (time_point.time_since_epoch ()).count ();
	rsnano::rsn_representative_set_last_request (handle, timepoint_ns);
}

std::chrono::system_clock::time_point nano::representative::get_last_response () const
{
	return rsnano::time_point_from_nanoseconds (rsnano::rsn_representative_last_response (handle));
}

void nano::representative::set_last_response (std::chrono::system_clock::time_point time_point)
{
	rsnano::rsn_representative_set_last_response (handle, std::chrono::nanoseconds (time_point.time_since_epoch ()).count ());
}

//------------------------------------------------------------------------------
// representative_register
//------------------------------------------------------------------------------

nano::representative_register::representative_register (nano::node & node_a) :
	node{ node_a }
{
	auto network_dto{ node_a.config->network_params.network.to_dto () };
	handle = rsnano::rsn_representative_register_create (
	node_a.ledger.handle,
	node_a.online_reps.get_handle (),
	&network_dto);
}

nano::representative_register::~representative_register ()
{
	rsnano::rsn_representative_register_destroy (handle);
}

nano::representative_register::insert_result nano::representative_register::update_or_insert (nano::account account_a, std::shared_ptr<nano::transport::channel> const & channel_a)
{
	rsnano::EndpointDto endpoint_dto;
	auto result_code = rsnano::rsn_representative_register_update_or_insert (handle, account_a.bytes.data (), channel_a->handle, &endpoint_dto);
	nano::representative_register::insert_result result{};
	if (result_code == 0)
	{
		result.inserted = true;
	}
	else if (result_code == 1)
	{
		// updated
	}
	else if (result_code == 2)
	{
		result.updated = true;
		result.prev_endpoint = rsnano::dto_to_endpoint (endpoint_dto);
	}
	else
	{
		throw std::runtime_error ("unknown result code");
	}
	return result;
}

bool nano::representative_register::is_pr (nano::transport::channel const & channel_a) const
{
	return rsnano::rsn_representative_register_is_pr (handle, channel_a.handle);
}

nano::uint128_t nano::representative_register::total_weight () const
{
	nano::amount result;
	rsnano::rsn_representative_register_total_weight (handle, result.bytes.data ());
	return result.number ();
}

void nano::representative_register::on_rep_request (std::shared_ptr<nano::transport::channel> const & channel_a)
{
	rsnano::rsn_representative_register_on_rep_request (handle, channel_a->handle);
}

void nano::representative_register::cleanup_reps ()
{
	rsnano::rsn_representative_register_cleanup_reps (handle);
}

std::vector<nano::representative> nano::representative_register::representatives (std::size_t count_a, nano::uint128_t const weight_a, boost::optional<decltype (nano::network_constants::protocol_version)> const & opt_version_min_a)
{
	uint8_t min_version = opt_version_min_a.value_or (0);
	nano::amount weight{ weight_a };

	auto result_handle = rsnano::rsn_representative_register_representatives (handle, count_a, weight.bytes.data (), min_version);

	auto len = rsnano::rsn_representative_list_len (result_handle);
	std::vector<nano::representative> result;
	result.reserve (len);
	for (auto i = 0; i < len; ++i)
	{
		result.emplace_back (rsnano::rsn_representative_list_get (result_handle, i));
	}
	rsnano::rsn_representative_list_destroy (result_handle);
	return result;
}

std::vector<nano::representative> nano::representative_register::principal_representatives (std::size_t count_a, boost::optional<decltype (nano::network_constants::protocol_version)> const & opt_version_min_a)
{
	return representatives (count_a, node.minimum_principal_weight (), opt_version_min_a);
}

/** Total number of representatives */
std::size_t nano::representative_register::representative_count ()
{
	return rsnano::rsn_representative_register_count (handle);
	//nano::lock_guard<nano::mutex> lock{ probable_reps_mutex };
	//return probable_reps.size ();
}

nano::rep_crawler::rep_crawler (nano::node & node_a) :
	node (node_a),
	handle{ rsnano::rsn_rep_crawler_create () }
{
	if (!node.flags.disable_rep_crawler ())
	{
		node.observers->endpoint.add ([this] (std::shared_ptr<nano::transport::channel> const & channel_a) {
			this->query (channel_a);
		});
	}
}

nano::rep_crawler::~rep_crawler ()
{
	rsnano::rsn_rep_crawler_destroy (handle);
}

void nano::rep_crawler::remove (nano::block_hash const & hash_a)
{
	rsnano::rsn_rep_crawler_remove (handle, hash_a.bytes.data ());
}

void nano::rep_crawler::start ()
{
	ongoing_crawl ();
}

void nano::rep_crawler::validate ()
{
	decltype (responses) responses_l;
	{
		nano::lock_guard<nano::mutex> lock{ active_mutex };
		responses_l.swap (responses);
	}

	// normally the rep_crawler only tracks principal reps but it can be made to track
	// reps with less weight by setting rep_crawler_weight_minimum to a low value
	auto minimum = std::min (node.minimum_principal_weight (), node.config->rep_crawler_weight_minimum.number ());

	for (auto const & i : responses_l)
	{
		auto & vote = i.second;
		auto & channel = i.first;
		debug_assert (channel != nullptr);

		if (channel->get_type () == nano::transport::transport_type::loopback)
		{
			node.logger->debug (nano::log::type::repcrawler, "Ignoring vote from loopback channel: {}", channel->to_string ());

			continue;
		}

		nano::uint128_t rep_weight = node.ledger.weight (vote->account ());
		if (rep_weight < minimum)
		{
			node.logger->debug (nano::log::type::repcrawler, "Ignoring vote from account {} with too little voting weight: {}",
			vote->account ().to_account (),
			nano::util::to_str (rep_weight));

			continue;
		}

		auto insert_result{ node.representative_register.update_or_insert (vote->account (), channel) };
		if (insert_result.inserted)
		{
			node.logger->info (nano::log::type::repcrawler, "Found representative {} at {}", vote->account ().to_account (), channel->to_string ());
		}

		if (insert_result.updated)
		{
			node.logger->warn (nano::log::type::repcrawler, "Updated representative {} at {} (was at: {})", vote->account ().to_account (), channel->to_string (), insert_result.prev_endpoint.address().to_string());
		}
	}
}

void nano::rep_crawler::ongoing_crawl ()
{
	auto total_weight_l (node.representative_register.total_weight ());
	node.representative_register.cleanup_reps ();
	validate ();
	query (get_crawl_targets (total_weight_l));
	auto sufficient_weight (total_weight_l > node.online_reps.delta ());
	// If online weight drops below minimum, reach out to preconfigured peers
	if (!sufficient_weight)
	{
		node.keepalive_preconfigured (node.config->preconfigured_peers);
	}
	// Reduce crawl frequency when there's enough total peer weight
	unsigned next_run_ms = node.network_params.network.is_dev_network () ? 100 : sufficient_weight ? 7000
																								   : 3000;
	std::weak_ptr<nano::node> node_w (node.shared ());
	auto now (std::chrono::steady_clock::now ());
	node.workers->add_timed_task (now + std::chrono::milliseconds (next_run_ms), [node_w, this] () {
		if (auto node_l = node_w.lock ())
		{
			this->ongoing_crawl ();
		}
	});
}

std::vector<std::shared_ptr<nano::transport::channel>> nano::rep_crawler::get_crawl_targets (nano::uint128_t total_weight_a)
{
	constexpr std::size_t conservative_count = 10;
	constexpr std::size_t aggressive_count = 40;

	// Crawl more aggressively if we lack sufficient total peer weight.
	bool sufficient_weight (total_weight_a > node.online_reps.delta ());
	uint16_t required_peer_count = sufficient_weight ? conservative_count : aggressive_count;

	// Add random peers. We do this even if we have enough weight, in order to pick up reps
	// that didn't respond when first observed. If the current total weight isn't sufficient, this
	// will be more aggressive. When the node first starts, the rep container is empty and all
	// endpoints will originate from random peers.
	required_peer_count += required_peer_count / 2;

	// The rest of the endpoints are picked randomly
	return node.network->tcp_channels->random_channels (required_peer_count, 0, true); // Include channels with ephemeral remote ports
}

void nano::rep_crawler::query (std::vector<std::shared_ptr<nano::transport::channel>> const & channels_a)
{
	auto transaction (node.store.tx_begin_read ());
	std::optional<std::pair<nano::block_hash, nano::block_hash>> hash_root;
	for (auto i = 0; i < 4 && !hash_root; ++i)
	{
		hash_root = node.ledger.hash_root_random (*transaction);
		if (node.active.recently_confirmed.exists (hash_root->first))
		{
			hash_root = std::nullopt;
		}
	}
	if (!hash_root)
	{
		return;
	}
	{
		nano::lock_guard<nano::mutex> lock{ active_mutex };
		// Don't send same block multiple times in tests
		if (node.network_params.network.is_dev_network ())
		{
			for (auto i (0); rsnano::rsn_rep_crawler_active_contains (handle, hash_root->first.bytes.data ()) && i < 4; ++i)
			{
				hash_root = node.ledger.hash_root_random (*transaction);
			}
		}
		rsnano::rsn_rep_crawler_active_insert (handle, hash_root->first.bytes.data ());
	}
	for (auto i (channels_a.begin ()), n (channels_a.end ()); i != n; ++i)
	{
		debug_assert (*i != nullptr);
		node.representative_register.on_rep_request (*i);
		// Confirmation request with hash + root
		nano::confirm_req req (node.network_params.network, hash_root->first, hash_root->second);
		(*i)->send (req);
	}

	// A representative must respond with a vote within the deadline
	std::weak_ptr<nano::node> node_w (node.shared ());
	node.workers->add_timed_task (std::chrono::steady_clock::now () + std::chrono::seconds (5), [node_w, hash = hash_root->first] () {
		if (auto node_l = node_w.lock ())
		{
			auto target_finished_processed (node_l->vote_processor.total_processed + node_l->vote_processor_queue.size ());
			node_l->rep_crawler.throttled_remove (hash, target_finished_processed);
		}
	});
}

void nano::rep_crawler::query (std::shared_ptr<nano::transport::channel> const & channel_a)
{
	std::vector<std::shared_ptr<nano::transport::channel>> peers;
	peers.emplace_back (channel_a);
	query (peers);
}

void nano::rep_crawler::throttled_remove (nano::block_hash const & hash_a, uint64_t const target_finished_processed)
{
	if (node.vote_processor.total_processed >= target_finished_processed)
	{
		remove (hash_a);
	}
	else
	{
		std::weak_ptr<nano::node> node_w (node.shared ());
		node.workers->add_timed_task (std::chrono::steady_clock::now () + std::chrono::seconds (5), [node_w, hash_a, target_finished_processed] () {
			if (auto node_l = node_w.lock ())
			{
				node_l->rep_crawler.throttled_remove (hash_a, target_finished_processed);
			}
		});
	}
}

void nano::rep_crawler::insert_active (nano::block_hash const & hash_a)
{
	rsnano::rsn_rep_crawler_active_insert (handle, hash_a.bytes.data ());
}

void nano::rep_crawler::insert_response (std::shared_ptr<nano::transport::channel> channel_a, std::shared_ptr<nano::vote> vote_a)
{
	rsnano::rsn_rep_crawler_response_insert (handle, channel_a->handle, vote_a->get_handle ());
}

bool nano::rep_crawler::response (std::shared_ptr<nano::transport::channel> const & channel_a, std::shared_ptr<nano::vote> const & vote_a, bool force)
{
	bool error = true;
	nano::lock_guard<nano::mutex> lock{ active_mutex };
	auto hashes = vote_a->hashes ();
	for (auto i = hashes.begin (), n = hashes.end (); i != n; ++i)
	{
		if (force || rsnano::rsn_rep_crawler_active_contains (handle, i->bytes.data ()))
		{
			responses.emplace_back (channel_a, vote_a);
			error = false;
			break;
		}
	}
	return error;
}

std::vector<std::shared_ptr<nano::transport::channel>> nano::rep_crawler::representative_endpoints (std::size_t count_a)
{
	std::vector<std::shared_ptr<nano::transport::channel>> result;
	auto reps (node.representative_register.representatives (count_a));
	for (auto const & rep : reps)
	{
		result.push_back (rep.get_channel ());
	}
	return result;
}
