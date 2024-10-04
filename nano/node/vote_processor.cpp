#include "nano/lib/rsnano.hpp"

#include <nano/lib/logging.hpp>
#include <nano/lib/stats.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/timer.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/node_observers.hpp>
#include <nano/node/nodeconfig.hpp>
#include <nano/node/rep_tiers.hpp>
#include <nano/node/repcrawler.hpp>
#include <nano/node/vote_processor.hpp>
#include <nano/secure/common.hpp>
#include <nano/secure/ledger.hpp>

using namespace std::chrono_literals;

/*
 * vote_processor_config
 */

nano::vote_processor_config::vote_processor_config (rsnano::VoteProcessorConfigDto const & dto) :
	max_pr_queue{ dto.max_pr_queue },
	max_non_pr_queue{ dto.max_non_pr_queue },
	pr_priority{ dto.pr_priority },
	threads{ dto.threads },
	batch_size{ dto.batch_size },
	max_triggered{ dto.max_triggered }
{
}

nano::error nano::vote_processor_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("max_pr_queue", max_pr_queue);
	toml.get ("max_non_pr_queue", max_non_pr_queue);
	toml.get ("pr_priority", pr_priority);
	toml.get ("threads", threads);
	toml.get ("batch_size", batch_size);

	return toml.get_error ();
}

rsnano::VoteProcessorConfigDto nano::vote_processor_config::to_dto () const
{
	return { max_pr_queue, max_non_pr_queue, pr_priority, threads, batch_size, max_triggered };
}
