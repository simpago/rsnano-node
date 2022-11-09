#pragma once

#include <nano/secure/store.hpp>

namespace nano
{
namespace lmdb
{
	class store;
	class version_store : public nano::version_store
	{
	protected:
		rsnano::LmdbVersionStoreHandle * handle;

	public:
		explicit version_store (rsnano::LmdbVersionStoreHandle * handle_a);
		~version_store ();
		version_store (version_store const &) = delete;
		version_store (version_store &&) = delete;
		void put (nano::write_transaction const & transaction_a, int version_a) override;
		int get (nano::transaction const & transaction_a) const override;
	};
}
}
