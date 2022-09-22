#pragma once

#include <nano/lib/errors.hpp>
#include <nano/lib/rsnano.hpp>

#include <chrono>

namespace nano
{
class jsonconfig;
class tomlconfig;
class txn_tracking_config final
{
public:
	txn_tracking_config ();
	void load_dto (rsnano::TxnTrackingConfigDto & dto);
	rsnano::TxnTrackingConfigDto to_dto () const;
	/** If true, enable tracking for transaction read/writes held open longer than the min time variables */
	bool enable;
	std::chrono::milliseconds min_read_txn_time;
	std::chrono::milliseconds min_write_txn_time;
	bool ignore_writes_below_block_processor_max_time;
};

/** Configuration options for diagnostics information */
class diagnostics_config final
{
public:
	rsnano::TxnTrackingConfigDto to_dto () const;
	void load_dto (rsnano::TxnTrackingConfigDto & dto);
	nano::error deserialize_toml (nano::tomlconfig &);

	txn_tracking_config txn_tracking;
};
}
