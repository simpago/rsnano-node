#include "nano/lib/rsnano.hpp"

#include <nano/lib/tomlconfig.hpp>
#include <nano/node/bootstrap/bootstrap_config.hpp>

#include <chrono>

/*
 * account_sets_config
 */
nano::account_sets_config::account_sets_config ()
{
	rsnano::AccountSetsConfigDto dto;
	rsnano::rsn_account_sets_config_create (&dto);
	load_dto (dto);
}

nano::account_sets_config::account_sets_config (rsnano::AccountSetsConfigDto const & dto_a)
{
	load_dto (dto_a);
}

rsnano::AccountSetsConfigDto nano::account_sets_config::to_dto () const
{
	rsnano::AccountSetsConfigDto dto;
	dto.consideration_count = consideration_count;
	dto.priorities_max = priorities_max;
	dto.blocking_max = blocking_max;
	dto.cooldown_ms = cooldown.count ();
	return dto;
}

void nano::account_sets_config::load_dto (rsnano::AccountSetsConfigDto const & dto)
{
	consideration_count = dto.consideration_count;
	priorities_max = dto.priorities_max;
	blocking_max = dto.blocking_max;
	cooldown = std::chrono::milliseconds{ dto.cooldown_ms };
}

nano::error nano::account_sets_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("consideration_count", consideration_count);
	toml.get ("priorities_max", priorities_max);
	toml.get ("blocking_max", blocking_max);
	toml.get_duration ("cooldown", cooldown);

	return toml.get_error ();
}

/*
 * bootstrap_ascending_config
 */
nano::bootstrap_ascending_config::bootstrap_ascending_config ()
{
	rsnano::BootstrapAscendingConfigDto dto;
	rsnano::rsn_bootstrap_config_create (&dto);
	load_dto (dto);
}

nano::bootstrap_ascending_config::bootstrap_ascending_config (rsnano::BootstrapAscendingConfigDto const & dto_a)
{
	load_dto (dto_a);
}

rsnano::BootstrapAscendingConfigDto nano::bootstrap_ascending_config::to_dto () const
{
	rsnano::BootstrapAscendingConfigDto dto;
	dto.database_requests_limit = database_requests_limit;
	dto.requests_limit = requests_limit;
	dto.pull_count = pull_count;
	dto.timeout_ms = request_timeout.count ();
	dto.throttle_coefficient = throttle_coefficient;
	dto.throttle_wait_ms = throttle_wait.count ();
	dto.block_wait_count = block_wait_count;
	dto.account_sets = account_sets.to_dto ();
	return dto;
}

void nano::bootstrap_ascending_config::load_dto (rsnano::BootstrapAscendingConfigDto const & dto)
{
	database_requests_limit = dto.database_requests_limit;
	requests_limit = dto.requests_limit;
	pull_count = dto.pull_count;
	request_timeout = std::chrono::milliseconds{ dto.timeout_ms };
	throttle_coefficient = dto.throttle_coefficient;
	throttle_wait = std::chrono::milliseconds{ dto.throttle_wait_ms };
	block_wait_count = dto.block_wait_count;
	account_sets.load_dto (dto.account_sets);
}

nano::error nano::bootstrap_ascending_config::deserialize (nano::tomlconfig & toml)
{
	toml.get ("requests_limit", requests_limit);
	toml.get ("database_requests_limit", database_requests_limit);
	toml.get ("pull_count", pull_count);
	toml.get_duration ("timeout", request_timeout);
	toml.get ("throttle_coefficient", throttle_coefficient);
	toml.get_duration ("throttle_wait", throttle_wait);
	toml.get ("block_wait_count", block_wait_count);

	if (toml.has_key ("account_sets"))
	{
		auto config_l = toml.get_required_child ("account_sets");
		account_sets.deserialize (config_l);
	}

	return toml.get_error ();
}
