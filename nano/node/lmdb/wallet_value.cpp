#include <nano/node/lmdb/wallet_value.hpp>

nano::wallet_value::wallet_value (nano::db_val<rsnano::MdbVal> const & val_a)
{
	debug_assert (val_a.size () == sizeof (*this));
	std::copy (reinterpret_cast<uint8_t const *> (val_a.data ()), reinterpret_cast<uint8_t const *> (val_a.data ()) + sizeof (key), key.chars.begin ());
	std::copy (reinterpret_cast<uint8_t const *> (val_a.data ()) + sizeof (key), reinterpret_cast<uint8_t const *> (val_a.data ()) + sizeof (key) + sizeof (work), reinterpret_cast<char *> (&work));
}

nano::wallet_value::wallet_value (nano::raw_key const & key_a, uint64_t work_a) :
	key (key_a),
	work (work_a)
{
}

rsnano::WalletValueDto nano::wallet_value::to_dto () const
{
	rsnano::WalletValueDto result;
	std::copy (std::begin (key.bytes), std::end (key.bytes), std::begin (result.key));
	result.work = work;
	return result;
}

nano::wallet_value::wallet_value (rsnano::WalletValueDto const & dto_a) :
	work{ dto_a.work }
{
	std::copy (std::begin (dto_a.key), std::end (dto_a.key), std::begin (key.bytes));
}
