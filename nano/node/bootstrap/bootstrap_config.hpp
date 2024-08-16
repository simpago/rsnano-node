#pragma once

#include <nano/lib/errors.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/timer.hpp>
#include <nano/node/bootstrap/bootstrap_server.hpp>

namespace nano
{
class tomlconfig;

class account_sets_config final
{
public:
	account_sets_config ();
	account_sets_config (rsnano::AccountSetsConfigDto const & dto_a);

	rsnano::AccountSetsConfigDto to_dto () const;
	void load_dto (rsnano::AccountSetsConfigDto const & dto_a);

	nano::error deserialize (nano::tomlconfig & toml);

	std::size_t consideration_count;
	std::size_t priorities_max;
	std::size_t blocking_max;
	std::chrono::milliseconds cooldown;
};

class bootstrap_ascending_config final
{
public:
	bootstrap_ascending_config ();
	bootstrap_ascending_config (rsnano::BootstrapAscendingConfigDto const & dto_a);

	rsnano::BootstrapAscendingConfigDto to_dto () const;
	void load_dto (rsnano::BootstrapAscendingConfigDto const & dto_a);

	nano::error deserialize (nano::tomlconfig & toml);

	// Maximum number of un-responded requests per channel
	std::size_t requests_limit;
	std::size_t database_requests_limit;
	std::size_t pull_count;
	std::chrono::milliseconds request_timeout;
	std::size_t throttle_coefficient;
	std::chrono::milliseconds throttle_wait;
	std::size_t block_wait_count;

	nano::account_sets_config account_sets;
};
}
