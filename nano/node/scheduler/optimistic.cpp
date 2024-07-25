#include "nano/lib/rsnano.hpp"

#include <nano/lib/blocks.hpp>
#include <nano/lib/stats.hpp>
#include <nano/lib/tomlconfig.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/election_behavior.hpp>
#include <nano/node/node.hpp>
#include <nano/node/scheduler/optimistic.hpp>
#include <nano/secure/ledger.hpp>

nano::scheduler::optimistic::optimistic (rsnano::OptimisticSchedulerHandle * handle) :
	handle{ handle }
{
}

nano::scheduler::optimistic::~optimistic ()
{
	rsnano::rsn_optimistic_scheduler_destroy (handle);
}

/*
 * optimistic_scheduler_config
 */

nano::scheduler::optimistic_config::optimistic_config ()
{
	rsnano::OptimisticSchedulerConfigDto dto;
	rsnano::rsn_optimistic_scheduler_config_create (&dto);
	load_dto (dto);
}

void nano::scheduler::optimistic_config::load_dto (rsnano::OptimisticSchedulerConfigDto const & dto_a)
{
	enabled = dto_a.enabled;
	gap_threshold = dto_a.gap_threshold;
	max_size = dto_a.max_size;
}

rsnano::OptimisticSchedulerConfigDto nano::scheduler::optimistic_config::into_dto () const
{
	rsnano::OptimisticSchedulerConfigDto dto;
	dto.enabled = enabled;
	dto.gap_threshold = gap_threshold;
	dto.max_size = max_size;
	return dto;
}

nano::error nano::scheduler::optimistic_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("enable", enabled);
	toml.get ("gap_threshold", gap_threshold);
	toml.get ("max_size", max_size);

	return toml.get_error ();
}
