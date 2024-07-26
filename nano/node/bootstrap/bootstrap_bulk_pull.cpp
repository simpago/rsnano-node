#include "nano/lib/blocks.hpp"
#include "nano/lib/epoch.hpp"
#include "nano/lib/rsnano.hpp"
#include "nano/lib/rsnanoutils.hpp"
#include "nano/node/messages.hpp"
#include "nano/secure/common.hpp"

#include <nano/node/bootstrap/bootstrap.hpp>
#include <nano/node/bootstrap/bootstrap_bulk_pull.hpp>
#include <nano/node/bootstrap/bootstrap_connections.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/tcp.hpp>
#include <nano/secure/ledger.hpp>

#include <boost/format.hpp>

nano::pull_info::pull_info (nano::hash_or_account const & account_or_head_a, nano::block_hash const & head_a, nano::block_hash const & end_a, uint64_t bootstrap_id_a, count_t count_a, unsigned retry_limit_a) :
	account_or_head (account_or_head_a),
	head (head_a),
	head_original (head_a),
	end (end_a),
	count (count_a),
	retry_limit (retry_limit_a),
	bootstrap_id (bootstrap_id_a)
{
}
rsnano::PullInfoDto nano::pull_info::to_dto () const
{
	rsnano::PullInfoDto dto;
	std::copy (std::begin (account_or_head.bytes), std::end (account_or_head.bytes), std::begin (dto.account_or_head));
	std::copy (std::begin (head.bytes), std::end (head.bytes), std::begin (dto.head));
	std::copy (std::begin (head_original.bytes), std::end (head_original.bytes), std::begin (dto.head_original));
	std::copy (std::begin (end.bytes), std::end (end.bytes), std::begin (dto.end));
	dto.count = count;
	dto.attempts = attempts;
	dto.processed = processed;
	dto.retry_limit = retry_limit;
	dto.bootstrap_id = bootstrap_id;
	return dto;
}

void nano::pull_info::load_dto (rsnano::PullInfoDto const & dto)
{
	std::copy (std::begin (dto.account_or_head), std::end (dto.account_or_head), std::begin (account_or_head.bytes));
	std::copy (std::begin (dto.head), std::end (dto.head), std::begin (head.bytes));
	std::copy (std::begin (dto.head_original), std::end (dto.head_original), std::begin (head_original.bytes));
	std::copy (std::begin (dto.end), std::end (dto.end), std::begin (end.bytes));
	count = dto.count;
	attempts = dto.attempts;
	processed = dto.processed;
	retry_limit = dto.retry_limit;
	bootstrap_id = dto.bootstrap_id;
}

nano::bulk_pull::count_t nano::bulk_pull_server::get_sent_count () const
{
	return rsnano::rsn_bulk_pull_server_sent_count (handle);
}

nano::bulk_pull::count_t nano::bulk_pull_server::get_max_count () const
{
	return rsnano::rsn_bulk_pull_server_max_count (handle);
}

nano::bulk_pull nano::bulk_pull_server::get_request () const
{
	return nano::bulk_pull{ rsnano::rsn_bulk_pull_server_request (handle) };
}

nano::block_hash nano::bulk_pull_server::get_current () const
{
	nano::block_hash current;
	rsnano::rsn_bulk_pull_server_current (handle, current.bytes.data ());
	return current;
}

std::shared_ptr<nano::block> nano::bulk_pull_server::get_next ()
{
	auto block_handle = rsnano::rsn_bulk_pull_server_get_next (handle);
	return nano::block_handle_to_block (block_handle);
}

nano::bulk_pull_server::bulk_pull_server (std::shared_ptr<nano::node> const & node_a, std::shared_ptr<nano::transport::tcp_server> const & connection_a, std::unique_ptr<nano::bulk_pull> request_a)
{
	handle = rsnano::rsn_bulk_pull_server_create (
	request_a->handle,
	connection_a->handle,
	node_a->ledger.handle,
	node_a->bootstrap_workers->handle,
	node_a->async_rt.handle);
}

nano::bulk_pull_server::~bulk_pull_server ()
{
	rsnano::rsn_bulk_pull_server_destroy (handle);
}

nano::bulk_pull_account_server::bulk_pull_account_server (std::shared_ptr<nano::node> const & node_a, std::shared_ptr<nano::transport::tcp_server> const & connection_a, std::unique_ptr<nano::bulk_pull_account> request_a)
{
	handle = rsnano::rsn_bulk_pull_account_server_create (
	request_a->handle,
	connection_a->handle,
	node_a->ledger.handle,
	node_a->bootstrap_workers->handle,
	node_a->async_rt.handle);
}

nano::bulk_pull_account_server::~bulk_pull_account_server ()
{
	rsnano::rsn_bulk_pull_account_server_destroy (handle);
}

std::pair<std::unique_ptr<nano::pending_key>, std::unique_ptr<nano::pending_info>> nano::bulk_pull_account_server::get_next ()
{
	std::pair<std::unique_ptr<nano::pending_key>, std::unique_ptr<nano::pending_info>> result;

	rsnano::PendingKeyDto key_dto;
	rsnano::PendingInfoDto info_dto;
	if (rsnano::rsn_bulk_pull_account_server_get_next (handle, &key_dto, &info_dto))
	{
		nano::account acc{};
		std::copy (std::begin (key_dto.account), std::end (key_dto.account), acc.bytes.data ());
		auto hash = nano::block_hash::from_bytes (&key_dto.hash[0]);
		result.first = std::make_unique<nano::pending_key> (acc, hash);

		nano::account source{};
		std::copy (std::begin (info_dto.source), std::end (info_dto.source), source.bytes.data ());
		nano::amount amount{};
		std::copy (std::begin (info_dto.amount), std::end (info_dto.amount), amount.bytes.data ());
		auto epoch = static_cast<nano::epoch> (info_dto.epoch);

		result.second = std::make_unique<nano::pending_info> (source, amount, epoch);
	}
	else
	{
		result.first = nullptr;
		result.second = nullptr;
	}

	return result;
}

nano::pending_key nano::bulk_pull_account_server::current_key ()
{
	rsnano::PendingKeyDto key_dto;
	rsnano::rsn_bulk_pull_account_server_current_key (handle, &key_dto);

	nano::account acc{};
	std::copy (std::begin (key_dto.account), std::end (key_dto.account), acc.bytes.data ());
	auto hash = nano::block_hash::from_bytes (&key_dto.hash[0]);
	return nano::pending_key{ acc, hash };
}

bool nano::bulk_pull_account_server::pending_address_only ()
{
	return rsnano::rsn_bulk_pull_account_server_pending_address_only (handle);
}

bool nano::bulk_pull_account_server::pending_include_address ()
{
	return rsnano::rsn_bulk_pull_account_server_pending_include_address (handle);
}

bool nano::bulk_pull_account_server::invalid_request ()
{
	return rsnano::rsn_bulk_pull_account_server_invalid_request (handle);
}
