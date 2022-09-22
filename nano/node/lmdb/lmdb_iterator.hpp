#pragma once

#include <nano/secure/store.hpp>

#include <lmdb/libraries/liblmdb/lmdb.h>

namespace nano
{
template <typename T, typename U>
class mdb_iterator : public store_iterator_impl<T, U>
{
public:
	mdb_iterator (nano::transaction const & transaction_a, MDB_dbi db_a, MDB_val const & val_a = MDB_val{}, bool const direction_asc = true) :
		handle{ rsnano::rsn_lmdb_iterator_create (static_cast<rsnano::MdbTxn *> (transaction_a.get_handle ()), db_a, reinterpret_cast<const rsnano::MdbVal *> (&val_a), direction_asc, sizeof (T)) }
	{
		load_current ();
	}

	mdb_iterator (rsnano::LmdbIteratorHandle * handle_a) :
		handle{ handle_a }
	{
		load_current ();
	}

	mdb_iterator () = default;

	mdb_iterator (nano::mdb_iterator<T, U> && other_a)
	{
		handle = other_a.handle;
		other_a.handle = nullptr;
		current = other_a.current;
	}

	mdb_iterator (nano::mdb_iterator<T, U> const &) = delete;

	~mdb_iterator ()
	{
		if (handle != nullptr)
		{
			rsnano::rsn_lmdb_iterator_destroy (handle);
		}
	}

	nano::store_iterator_impl<T, U> & operator++ () override
	{
		rsnano::rsn_lmdb_iterator_next (handle);
		load_current ();
		return *this;
	}

	nano::store_iterator_impl<T, U> & operator-- () override
	{
		rsnano::rsn_lmdb_iterator_previous (handle);
		load_current ();
		return *this;
	}

	std::pair<nano::db_val<MDB_val>, nano::db_val<MDB_val>> * operator-> ()
	{
		return &current;
	}

	bool operator== (nano::store_iterator_impl<T, U> const & base_a) const override
	{
		auto const other_a (boost::polymorphic_downcast<nano::mdb_iterator<T, U> const *> (&base_a));
		auto result (current.first.data () == other_a->current.first.data ());
		debug_assert (!result || (current.first.size () == other_a->current.first.size ()));
		debug_assert (!result || (current.second.data () == other_a->current.second.data ()));
		debug_assert (!result || (current.second.size () == other_a->current.second.size ()));
		return result;
	}

	bool is_end_sentinal () const override
	{
		return current.first.size () == 0;
	}
	void fill (std::pair<T, U> & value_a) const override
	{
		if (current.first.size () != 0)
		{
			value_a.first = static_cast<T> (current.first);
		}
		else
		{
			value_a.first = T ();
		}
		if (current.second.size () != 0)
		{
			value_a.second = static_cast<U> (current.second);
		}
		else
		{
			value_a.second = U ();
		}
	}
	void clear ()
	{
		rsnano::rsn_lmdb_iterator_clear (handle);
		load_current ();
	}

	nano::mdb_iterator<T, U> & operator= (nano::mdb_iterator<T, U> && other_a)
	{
		if (handle != nullptr)
		{
			rsnano::rsn_lmdb_iterator_destroy (handle);
		}
		handle = other_a.handle;
		other_a.handle = nullptr;
		current = other_a.current;
		return *this;
	}

	nano::store_iterator_impl<T, U> & operator= (nano::store_iterator_impl<T, U> const &) = delete;
	MDB_cursor * get_cursor ()
	{
		return reinterpret_cast<MDB_cursor *> (rsnano::rsn_lmdb_iterator_cursor (handle));
	}
	std::pair<nano::db_val<MDB_val>, nano::db_val<MDB_val>> get_current ()
	{
		return current;
	}

private:
	rsnano::LmdbIteratorHandle * handle{ nullptr };
	std::pair<nano::db_val<MDB_val>, nano::db_val<MDB_val>> current;

	void load_current ()
	{
		rsnano::rsn_lmdb_iterator_current (handle, reinterpret_cast<rsnano::MdbVal *> (&current.first.value), reinterpret_cast<rsnano::MdbVal *> (&current.second.value));
	}
};

/**
 * Iterates the key/value pairs of two stores merged together
 */
template <typename T, typename U>
class mdb_merge_iterator : public store_iterator_impl<T, U>
{
public:
	mdb_merge_iterator (nano::transaction const & transaction_a, MDB_dbi db1_a, MDB_dbi db2_a) :
		impl1 (std::make_unique<nano::mdb_iterator<T, U>> (transaction_a, db1_a)),
		impl2 (std::make_unique<nano::mdb_iterator<T, U>> (transaction_a, db2_a))
	{
	}

	mdb_merge_iterator () :
		impl1 (std::make_unique<nano::mdb_iterator<T, U>> ()),
		impl2 (std::make_unique<nano::mdb_iterator<T, U>> ())
	{
	}

	mdb_merge_iterator (nano::transaction const & transaction_a, MDB_dbi db1_a, MDB_dbi db2_a, MDB_val const & val_a) :
		impl1 (std::make_unique<nano::mdb_iterator<T, U>> (transaction_a, db1_a, val_a)),
		impl2 (std::make_unique<nano::mdb_iterator<T, U>> (transaction_a, db2_a, val_a))
	{
	}

	mdb_merge_iterator (nano::mdb_merge_iterator<T, U> && other_a)
	{
		impl1 = std::move (other_a.impl1);
		impl2 = std::move (other_a.impl2);
	}

	mdb_merge_iterator (nano::mdb_merge_iterator<T, U> const &) = delete;

	nano::store_iterator_impl<T, U> & operator++ () override
	{
		++least_iterator ();
		return *this;
	}

	nano::store_iterator_impl<T, U> & operator-- () override
	{
		--least_iterator ();
		return *this;
	}

	std::pair<nano::db_val<MDB_val>, nano::db_val<MDB_val>> * operator-> ()
	{
		return least_iterator ().operator-> ();
	}

	bool operator== (nano::store_iterator_impl<T, U> const & base_a) const override
	{
		debug_assert ((dynamic_cast<nano::mdb_merge_iterator<T, U> const *> (&base_a) != nullptr) && "Incompatible iterator comparison");
		auto & other (static_cast<nano::mdb_merge_iterator<T, U> const &> (base_a));
		return *impl1 == *other.impl1 && *impl2 == *other.impl2;
	}

	bool is_end_sentinal () const override
	{
		return least_iterator ().is_end_sentinal ();
	}

	void fill (std::pair<T, U> & value_a) const override
	{
		auto & current (least_iterator ());
		if (current->first.size () != 0)
		{
			value_a.first = static_cast<T> (current->first);
		}
		else
		{
			value_a.first = T ();
		}
		if (current->second.size () != 0)
		{
			value_a.second = static_cast<U> (current->second);
		}
		else
		{
			value_a.second = U ();
		}
	}
	nano::mdb_merge_iterator<T, U> & operator= (nano::mdb_merge_iterator<T, U> &&) = default;
	nano::mdb_merge_iterator<T, U> & operator= (nano::mdb_merge_iterator<T, U> const &) = delete;

	mutable bool from_first_database{ false };

private:
	nano::mdb_iterator<T, U> & least_iterator () const
	{
		nano::mdb_iterator<T, U> * result;
		if (impl1->is_end_sentinal ())
		{
			result = impl2.get ();
			from_first_database = false;
		}
		else if (impl2->is_end_sentinal ())
		{
			result = impl1.get ();
			from_first_database = true;
		}
		else
		{
			auto key_cmp (mdb_cmp (mdb_cursor_txn (impl1->get_cursor ()), mdb_cursor_dbi (impl1->get_cursor ()), impl1->get_current ().first, impl2->get_current ().first));

			if (key_cmp < 0)
			{
				result = impl1.get ();
				from_first_database = true;
			}
			else if (key_cmp > 0)
			{
				result = impl2.get ();
				from_first_database = false;
			}
			else
			{
				auto val_cmp (mdb_cmp (mdb_cursor_txn (impl1->get_cursor ()), mdb_cursor_dbi (impl1->get_cursor ()), impl1->get_current ().second, impl2->get_current ().second));
				result = val_cmp < 0 ? impl1.get () : impl2.get ();
				from_first_database = (result == impl1.get ());
			}
		}
		return *result;
	}

	std::unique_ptr<nano::mdb_iterator<T, U>> impl1;
	std::unique_ptr<nano::mdb_iterator<T, U>> impl2;
};
}
