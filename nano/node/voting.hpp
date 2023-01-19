#pragma once

#include <nano/lib/locks.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/processing_queue.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/wallet.hpp>
#include <nano/secure/common.hpp>

#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/member.hpp>
#include <boost/multi_index/ordered_index.hpp>
#include <boost/multi_index/sequenced_index.hpp>
#include <boost/multi_index_container.hpp>

#include <condition_variable>
#include <deque>
#include <thread>

namespace mi = boost::multi_index;

namespace nano
{
class ledger;
class network;
class node_config;
class stat;
class vote_processor;
class wallets;
namespace transport
{
	class channel;
}

class vote_spacing final
{
public:
	vote_spacing (std::chrono::milliseconds const & delay);
	vote_spacing (vote_spacing const &) = delete;
	vote_spacing (vote_spacing &&) = delete;
	~vote_spacing ();
	bool votable (nano::root const & root_a, nano::block_hash const & hash_a) const;
	void flag (nano::root const & root_a, nano::block_hash const & hash_a);
	std::size_t size () const;
	rsnano::VoteSpacingHandle * handle;
};

class local_vote_history final
{
public:
	local_vote_history (nano::voting_constants const & constants);
	local_vote_history (const local_vote_history &) = delete;
	local_vote_history (local_vote_history &&) = delete;
	~local_vote_history ();
	void add (nano::root const & root_a, nano::block_hash const & hash_a, std::shared_ptr<nano::vote> const & vote_a);
	void erase (nano::root const & root_a);

	std::vector<std::shared_ptr<nano::vote>> votes (nano::root const & root_a, nano::block_hash const & hash_a, bool const is_final_a = false) const;
	bool exists (nano::root const &) const;
	std::size_t size () const;

private:
	rsnano::LocalVoteHistoryHandle * handle;
	friend std::unique_ptr<container_info_component> collect_container_info (local_vote_history & history, std::string const & name);
	friend class local_vote_history_basic_Test;
};

std::unique_ptr<container_info_component> collect_container_info (local_vote_history & history, std::string const & name);

/** Floods a vote to the network and calls the vote processor. */
class vote_broadcaster
{
public:
	vote_broadcaster (nano::vote_processor & vote_processor_a, nano::network & network_a);
	void broadcast (std::shared_ptr<nano::vote> const &) const;

private:
	nano::vote_processor & vote_processor;
	nano::network & network;
};

class vote_generator final
{
private:
	using candidate_t = std::pair<nano::root, nano::block_hash>;
	using request_t = std::pair<std::vector<candidate_t>, std::shared_ptr<nano::transport::channel>>;
	using queue_entry_t = std::pair<nano::root, nano::block_hash>;

public:
	vote_generator (nano::node_config const & config_a, nano::ledger & ledger_a, nano::wallets & wallets_a, nano::vote_processor & vote_processor_a, nano::local_vote_history & history_a, nano::network & network_a, nano::stat & stats_a, bool is_final_a);
	~vote_generator ();

	/** Queue items for vote generation, or broadcast votes already in cache */
	void add (nano::root const &, nano::block_hash const &);
	/** Queue blocks for vote generation, returning the number of successful candidates.*/
	std::size_t generate (std::vector<std::shared_ptr<nano::block>> const & blocks_a, std::shared_ptr<nano::transport::channel> const & channel_a);
	void set_reply_action (std::function<void (std::shared_ptr<nano::vote> const &, std::shared_ptr<nano::transport::channel> const &)>);

	void start ();
	void stop ();

private:
	void run ();
	void broadcast (nano::unique_lock<nano::mutex> &);
	void reply (nano::unique_lock<nano::mutex> &, request_t &&);
	void vote (std::vector<nano::block_hash> const &, std::vector<nano::root> const &, std::function<void (std::shared_ptr<nano::vote> const &)> const &);
	void broadcast_action (std::shared_ptr<nano::vote> const &) const;
	void process_batch (std::deque<queue_entry_t> & batch);
	/**
	 * Check if block is eligible for vote generation, then generates a vote or broadcasts votes already in cache
	 * @param transaction : needs `tables::final_votes` lock
	 */
	void process (nano::write_transaction const &, nano::root const &, nano::block_hash const &);
	std::function<void (std::shared_ptr<nano::vote> const &, std::shared_ptr<nano::transport::channel> &)> reply_action; // must be set only during initialization by using set_reply_action

	// already ported to Rust:
	nano::node_config const & config;
	nano::local_vote_history & history;
	nano::stat & stats;
	nano::vote_spacing spacing;

	// not ported yet:
	nano::ledger & ledger;
	nano::wallets & wallets;
	nano::vote_broadcaster vote_broadcaster;
	processing_queue<queue_entry_t> vote_generation_queue;
	const bool is_final;
	mutable nano::mutex mutex;
	nano::condition_variable condition;
	static std::size_t constexpr max_requests{ 2048 };
	std::deque<request_t> requests;
	std::deque<candidate_t> candidates;
	std::atomic<bool> stopped{ false };
	std::thread thread;

	friend std::unique_ptr<container_info_component> collect_container_info (vote_generator & vote_generator, std::string const & name);
};

std::unique_ptr<container_info_component> collect_container_info (vote_generator & generator, std::string const & name);
}
