#include <nano/lib/rsnano.hpp>
#include <nano/node/write_database_queue.hpp>

nano::write_guard::write_guard (rsnano::WriteGuardHandle * handle_a) :
	handle{ handle_a }
{
}

nano::write_guard::write_guard (nano::write_guard && write_guard_a) noexcept :
	handle (write_guard_a.handle),
	owns (write_guard_a.owns)
{
	write_guard_a.owns = false;
	write_guard_a.handle = nullptr;
}

nano::write_guard & nano::write_guard::operator= (nano::write_guard && write_guard_a) noexcept
{
	if (owns && handle != nullptr)
		rsnano::rsn_write_guard_destroy (handle);

	owns = write_guard_a.owns;
	handle = write_guard_a.handle;

	write_guard_a.owns = false;
	write_guard_a.handle = nullptr;
	return *this;
}

nano::write_guard::~write_guard ()
{
	if (owns)
	{
		rsnano::rsn_write_guard_destroy (handle);
	}
}

bool nano::write_guard::is_owned () const
{
	return owns;
}

void nano::write_guard::release ()
{
	if (owns)
	{
		rsnano::rsn_write_guard_release (handle);
	}
	owns = false;
}

nano::write_database_queue::write_database_queue (bool use_noops_a) :
	handle{ rsnano::rsn_write_database_queue_create (use_noops_a) }
{
}

nano::write_database_queue::~write_database_queue ()
{
	rsnano::rsn_write_database_queue_destroy (handle);
}

nano::write_guard nano::write_database_queue::wait (nano::writer writer)
{
	auto guard_handle = rsnano::rsn_write_database_queue_wait (handle, static_cast<uint8_t> (writer));
	return nano::write_guard (guard_handle);
}

bool nano::write_database_queue::contains (nano::writer writer)
{
	return rsnano::rsn_write_database_queue_contains (handle, static_cast<uint8_t> (writer));
}

bool nano::write_database_queue::process (nano::writer writer)
{
	return rsnano::rsn_write_database_queue_process (handle, static_cast<uint8_t> (writer));
}

nano::write_guard nano::write_database_queue::pop ()
{
	return nano::write_guard (rsnano::rsn_write_database_queue_pop (handle));
}
