#pragma once

#include <nano/lib/rsnano.hpp>
#include <nano/node/messages.hpp>
#include <nano/secure/pending_info.hpp>

namespace nano
{
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
}
