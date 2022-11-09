#pragma once

#include <nano/secure/store.hpp>

namespace nano
{
template <typename T, typename U>
class mdb_iterator : public store_iterator_impl<T, U>
{
public:
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

private:
	rsnano::LmdbIteratorHandle * handle{ nullptr };
	std::pair<nano::db_val<rsnano::MdbVal>, nano::db_val<rsnano::MdbVal>> current;

	void load_current ()
	{
		rsnano::rsn_lmdb_iterator_current (handle, reinterpret_cast<rsnano::MdbVal *> (&current.first.value), reinterpret_cast<rsnano::MdbVal *> (&current.second.value));
	}
};
}
