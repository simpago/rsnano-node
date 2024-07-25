#pragma once

#include "nano/lib/rsnano.hpp"

#include <nano/lib/locks.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/secure/common.hpp>
#include <nano/store/transaction.hpp>

#include <chrono>

namespace nano
{
class node;
class node_config;
class active_elections;
class vote_cache;
}

namespace nano::scheduler
{
class hinted_config final
{
public:
	hinted_config ();
	explicit hinted_config (nano::network_constants const &);

	void load_dto (rsnano::HintedSchedulerConfigDto const & dto_a);
	rsnano::HintedSchedulerConfigDto into_dto () const;
	nano::error deserialize (nano::tomlconfig & toml);
	nano::error serialize (nano::tomlconfig & toml) const;

public:
	bool enabled;
	std::chrono::milliseconds check_interval;
	std::chrono::milliseconds block_cooldown;
	unsigned hinting_threshold_percent;
	unsigned vacancy_threshold_percent;
};

/*
 * Monitors inactive vote cache and schedules elections with the highest observed vote tally.
 */
class hinted final
{
public:
	hinted (rsnano::HintedSchedulerHandle * handle);
	hinted (hinted const &) = delete;
	~hinted ();

	rsnano::HintedSchedulerHandle * handle;
};
}
