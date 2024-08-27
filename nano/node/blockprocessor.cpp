#include "nano/lib/blocks.hpp"
#include "nano/lib/rsnano.hpp"

#include <nano/lib/threading.hpp>
#include <nano/lib/timer.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/blockprocessor.hpp>
#include <nano/node/local_vote_history.hpp>
#include <nano/node/node.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/store/component.hpp>

#include <boost/format.hpp>

#include <cstdint>
#include <memory>

/*
 * block_processor
 */

nano::block_processor::block_processor (rsnano::BlockProcessorHandle * handle) :
	handle{ handle }
{
}

nano::block_processor::~block_processor ()
{
	rsnano::rsn_block_processor_destroy (handle);
}

rsnano::BlockProcessorHandle const * nano::block_processor::get_handle () const
{
	return handle;
}

void nano::block_processor::process_active (std::shared_ptr<nano::block> const & incoming)
{
	add (incoming);
}

bool nano::block_processor::add (std::shared_ptr<nano::block> const & block, block_source const source)
{
	return rsnano::rsn_block_processor_add (handle, block->get_handle (), static_cast<uint8_t> (source));
}

std::optional<nano::block_status> nano::block_processor::add_blocking (std::shared_ptr<nano::block> const & block, block_source const source)
{
	std::uint8_t status;
	if (rsnano::rsn_block_processor_add_blocking (handle, block->get_handle (), static_cast<uint8_t> (source), &status))
	{
		return static_cast<nano::block_status> (status);
	}
	else
	{
		return std::nullopt;
	}
}

void nano::block_processor::force (std::shared_ptr<nano::block> const & block_a)
{
	rsnano::rsn_block_processor_force (handle, block_a->get_handle ());
}

/*
 * block_processor_config
 */

nano::block_processor_config::block_processor_config (rsnano::BlockProcessorConfigDto const & dto)
{
	max_peer_queue = dto.max_peer_queue;
	max_system_queue = dto.max_system_queue;
	priority_live = dto.priority_live;
	priority_bootstrap = dto.priority_bootstrap;
	priority_local = dto.priority_local;
}

rsnano::BlockProcessorConfigDto nano::block_processor_config::to_dto () const
{
	rsnano::BlockProcessorConfigDto dto;
	dto.max_peer_queue = max_peer_queue;
	dto.max_system_queue = max_system_queue;
	dto.priority_live = priority_live;
	dto.priority_bootstrap = priority_bootstrap;
	dto.priority_local = priority_local;
	return dto;
}

nano::error nano::block_processor_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("max_peer_queue", max_peer_queue);
	toml.get ("max_system_queue", max_system_queue);
	toml.get ("priority_live", priority_live);
	toml.get ("priority_bootstrap", priority_bootstrap);
	toml.get ("priority_local", priority_local);

	return toml.get_error ();
}
