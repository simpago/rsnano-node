#pragma once

#include <nano/lib/rsnano.hpp>
#include <nano/node/messages.hpp>
#include <nano/node/transport/socket.hpp>
#include <nano/secure/pending_info.hpp>

namespace nano
{
class logger;
namespace transport
{
	class tcp_server;
}

class pull_info
{
public:
	using count_t = nano::bulk_pull::count_t;
	pull_info () = default;
	pull_info (nano::hash_or_account const &, nano::block_hash const &, nano::block_hash const &, uint64_t, count_t = 0, unsigned = 16);
	nano::hash_or_account account_or_head{ 0 };
	nano::block_hash head{ 0 };
	nano::block_hash head_original{ 0 };
	nano::block_hash end{ 0 };
	count_t count{ 0 };
	unsigned attempts{ 0 };
	uint64_t processed{ 0 };
	unsigned retry_limit{ 0 };
	uint64_t bootstrap_id{ 0 };
	rsnano::PullInfoDto to_dto () const;
	void load_dto (rsnano::PullInfoDto const & dto);
};
class bootstrap_client;
class bootstrap_connections;

class bulk_pull;

/**
 * Server side of a bulk_pull request. Created when tcp_server receives a bulk_pull message and is exited after the contents
 * have been sent. If the 'start' in the bulk_pull message is an account, send blocks for that account down to 'end'. If the 'start'
 * is a block hash, send blocks for that chain down to 'end'. If end doesn't exist, send all accounts in the chain.
 */
class bulk_pull_server final : public std::enable_shared_from_this<nano::bulk_pull_server>
{
public:
	bulk_pull_server (std::shared_ptr<nano::node> const &, std::shared_ptr<nano::transport::tcp_server> const &, std::unique_ptr<nano::bulk_pull>);
	bulk_pull_server (bulk_pull_server const &) = delete;
	~bulk_pull_server ();
	std::shared_ptr<nano::block> get_next ();
	nano::block_hash get_current () const;
	nano::bulk_pull::count_t get_max_count () const;
	nano::bulk_pull::count_t get_sent_count () const;
	nano::bulk_pull get_request () const;
	rsnano::BulkPullServerHandle * handle;
};
class bulk_pull_account;
class bulk_pull_account_server final : public std::enable_shared_from_this<nano::bulk_pull_account_server>
{
public:
	bulk_pull_account_server (std::shared_ptr<nano::node> const &, std::shared_ptr<nano::transport::tcp_server> const &, std::unique_ptr<nano::bulk_pull_account>);
	bulk_pull_account_server (bulk_pull_server const &) = delete;
	~bulk_pull_account_server ();
	std::pair<std::unique_ptr<nano::pending_key>, std::unique_ptr<nano::pending_info>> get_next ();
	nano::pending_key current_key ();
	bool pending_address_only ();
	bool pending_include_address ();
	bool invalid_request ();
	rsnano::BulkPullAccountServerHandle * handle;
};
}
