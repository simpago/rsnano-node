#pragma once

#include "nano/lib/rsnano.hpp"
#include "nano/node/common.hpp"

#include <nano/lib/locks.hpp>
#include <nano/node/transport/channel.hpp>
#include <nano/node/transport/transport.hpp>

#include <chrono>
#include <memory>

namespace nano
{
class node;

/**
 * A representative picked up during repcrawl.
 */
class representative
{
public:
	representative (nano::account account_a, std::shared_ptr<nano::transport::channel> const & channel_a);
	representative (representative const & other_a);
	representative (rsnano::RepresentativeHandle * handle_a);
	~representative ();
	representative & operator= (representative const & other_a);
	size_t channel_id () const;
	bool operator== (nano::representative const & other_a) const
	{
		return get_account () == other_a.get_account ();
	}
	nano::account get_account () const;

	rsnano::RepresentativeHandle * handle;
};

class rep_crawler_config final
{
public:
	explicit rep_crawler_config (std::chrono::milliseconds query_timeout_a);
	nano::error deserialize (nano::tomlconfig & toml);

public:
	std::chrono::milliseconds query_timeout;
};

class representative_register
{
public:
	class insert_result
	{
	public:
		bool inserted{ false };
		bool updated{ false };
		nano::tcp_endpoint prev_endpoint{};
	};

	representative_register (rsnano::RepresentativeRegisterHandle * handle);
	representative_register (representative_register const &) = delete;
	~representative_register ();

	void update_or_insert (nano::account account_a, std::shared_ptr<nano::transport::channel> const & channel_a);
	/** Query if a peer manages a principle representative */
	bool is_pr (std::shared_ptr<nano::transport::channel> const & target_channel) const;
	/** Get total available weight from representatives */
	nano::uint128_t total_weight () const;

	/** Request a list of the top \p count known representatives in descending order of weight, with at least \p mininum_weight voting weight, and optionally with a minimum version \p minimum_protocol_version
	 */
	std::vector<nano::representative> representatives (std::size_t count = std::numeric_limits<std::size_t>::max (), nano::uint128_t const minimum_weight = 0);

	/** Total number of representatives */
	std::size_t representative_count ();

	rsnano::RepresentativeRegisterHandle * handle;
};

/**
 * Crawls the network for representatives. Queries are performed by requesting confirmation of a
 * random block and observing the corresponding vote.
 */
class rep_crawler final
{
public:
	rep_crawler (rsnano::RepCrawlerHandle * handle, nano::node & node_a);
	rep_crawler (rep_crawler const &) = delete;
	~rep_crawler ();

	void start ();
	void stop ();

	/**
	 * Called when a non-replay vote arrives that might be of interest to rep crawler.
	 * @return true, if the vote was of interest and was processed, this indicates that the rep is likely online and voting
	 */
	bool process (std::shared_ptr<nano::vote> const &, std::shared_ptr<nano::transport::channel> const &);

	/** Attempt to determine if the peer manages one or more representative accounts */
	void query (std::shared_ptr<nano::transport::channel> const & target_channel);

	/** Query if a peer manages a principle representative */
	bool is_pr (std::shared_ptr<nano::transport::channel> const &) const;

	/** Total number of representatives */
	std::size_t representative_count ();

private:
	nano::node & node;

public:
	rsnano::RepCrawlerHandle * handle;

public: // Testing
	void force_add_rep (nano::account const & account, std::shared_ptr<nano::transport::channel> const & channel);
	void force_process (std::shared_ptr<nano::vote> const & vote, std::shared_ptr<nano::transport::channel> const & channel);
	void force_query (nano::block_hash const & hash, std::shared_ptr<nano::transport::channel> const & channel);
};
}
