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

#include <memory>

using namespace std::chrono_literals;

nano::vote_processor_queue::vote_processor_queue (rsnano::VoteProcessorQueueHandle * handle) :
	handle{ handle }
{
}

nano::vote_processor_queue::~vote_processor_queue ()
{
	rsnano::rsn_vote_processor_queue_destroy (handle);
}

bool nano::vote_processor_queue::vote (std::shared_ptr<nano::vote> const & vote_a, std::shared_ptr<nano::transport::channel> const & channel_a)
{
	return rsnano::rsn_vote_processor_queue_vote (handle, vote_a->get_handle (), channel_a->handle, static_cast<uint8_t> (nano::vote_source::live));
}

nano::vote_processor::vote_processor (rsnano::VoteProcessorHandle * handle) :
	handle{ handle }
{
}

nano::vote_processor::~vote_processor ()
{
	rsnano::rsn_vote_processor_destroy (handle);
}

nano::vote_code nano::vote_processor::vote_blocking (std::shared_ptr<nano::vote> const & vote, std::shared_ptr<nano::transport::channel> const & channel_a)
{
	return static_cast<nano::vote_code> (rsnano::rsn_vote_processor_vote_blocking (handle, vote->get_handle (), channel_a->handle, static_cast<uint8_t> (nano::vote_source::live)));
}

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
