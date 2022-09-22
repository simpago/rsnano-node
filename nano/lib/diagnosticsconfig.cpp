#include <nano/lib/diagnosticsconfig.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/tomlconfig.hpp>

nano::txn_tracking_config::txn_tracking_config ()
{
	rsnano::TxnTrackingConfigDto dto;
	rsnano::rsn_txn_tracking_config_create (&dto);
	load_dto (dto);
}

void nano::txn_tracking_config::load_dto (rsnano::TxnTrackingConfigDto & dto)
{
	enable = dto.enable;
	min_read_txn_time = std::chrono::milliseconds (dto.min_read_txn_time_ms);
	min_write_txn_time = std::chrono::milliseconds (dto.min_write_txn_time_ms);
	ignore_writes_below_block_processor_max_time = dto.ignore_writes_below_block_processor_max_time;
}

rsnano::TxnTrackingConfigDto nano::txn_tracking_config::to_dto () const
{
	rsnano::TxnTrackingConfigDto dto;
	dto.enable = enable;
	dto.min_read_txn_time_ms = min_read_txn_time.count ();
	dto.min_write_txn_time_ms = min_write_txn_time.count ();
	dto.ignore_writes_below_block_processor_max_time = ignore_writes_below_block_processor_max_time;
	return dto;
}

rsnano::TxnTrackingConfigDto nano::diagnostics_config::to_dto () const
{
	return txn_tracking.to_dto ();
}

void nano::diagnostics_config::load_dto (rsnano::TxnTrackingConfigDto & dto)
{
	txn_tracking.load_dto (dto);
}

nano::error nano::diagnostics_config::deserialize_toml (nano::tomlconfig & toml)
{
	auto txn_tracking_l (toml.get_optional_child ("txn_tracking"));
	if (txn_tracking_l)
	{
		txn_tracking_l->get_optional<bool> ("enable", txn_tracking.enable);
		auto min_read_txn_time_l = static_cast<unsigned long> (txn_tracking.min_read_txn_time.count ());
		txn_tracking_l->get_optional ("min_read_txn_time", min_read_txn_time_l);
		txn_tracking.min_read_txn_time = std::chrono::milliseconds (min_read_txn_time_l);

		auto min_write_txn_time_l = static_cast<unsigned long> (txn_tracking.min_write_txn_time.count ());
		txn_tracking_l->get_optional ("min_write_txn_time", min_write_txn_time_l);
		txn_tracking.min_write_txn_time = std::chrono::milliseconds (min_write_txn_time_l);

		txn_tracking_l->get_optional<bool> ("ignore_writes_below_block_processor_max_time", txn_tracking.ignore_writes_below_block_processor_max_time);
	}
	return toml.get_error ();
}
