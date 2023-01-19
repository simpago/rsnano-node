#include <nano/lib/epoch.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/utility.hpp>

#include <algorithm>

nano::epochs::epochs () :
	handle (rsnano::rsn_epochs_create ())
{
}

nano::epochs::epochs (nano::epochs && other) :
	handle (other.handle)
{
	other.handle = nullptr;
}

nano::epochs::~epochs ()
{
	if (handle != nullptr)
		rsnano::rsn_epochs_destroy (handle);
}

nano::epochs & nano::epochs::operator= (nano::epochs && other)
{
	if (handle != nullptr)
		rsnano::rsn_epochs_destroy (handle);
	handle = other.handle;
	other.handle = nullptr;
	return *this;
}

rsnano::EpochsHandle * nano::epochs::get_handle () const
{
	return handle;
}

nano::link nano::epochs::link (nano::epoch epoch_a) const
{
	nano::link link;
	rsnano::rsn_epochs_link (handle, static_cast<uint8_t> (epoch_a), link.bytes.data ());
	return link;
}

bool nano::epochs::is_epoch_link (nano::link const & link_a) const
{
	return rsnano::rsn_epochs_is_epoch_link (handle, link_a.bytes.data ());
}

nano::public_key nano::epochs::signer (nano::epoch epoch_a) const
{
	nano::public_key signer;
	rsnano::rsn_epochs_signer (handle, static_cast<uint8_t> (epoch_a), signer.bytes.data ());
	return signer;
}

nano::epoch nano::epochs::epoch (nano::link const & link_a) const
{
	return static_cast<nano::epoch> (rsnano::rsn_epochs_epoch (handle, link_a.bytes.data ()));
}

void nano::epochs::add (nano::epoch epoch_a, nano::public_key const & signer_a, nano::link const & link_a)
{
	rsnano::rsn_epochs_add (handle, static_cast<uint8_t> (epoch_a), signer_a.bytes.data (), link_a.bytes.data ());
}

bool nano::epochs::is_sequential (nano::epoch epoch_a, nano::epoch new_epoch_a)
{
	auto head_epoch = std::underlying_type_t<nano::epoch> (epoch_a);
	bool is_valid_epoch (head_epoch >= std::underlying_type_t<nano::epoch> (nano::epoch::epoch_0));
	return is_valid_epoch && (std::underlying_type_t<nano::epoch> (new_epoch_a) == (head_epoch + 1));
}

std::underlying_type_t<nano::epoch> nano::normalized_epoch (nano::epoch epoch_a)
{
	// Currently assumes that the epoch versions in the enum are sequential.
	auto start = std::underlying_type_t<nano::epoch> (nano::epoch::epoch_0);
	auto end = std::underlying_type_t<nano::epoch> (epoch_a);
	debug_assert (end >= start);
	return end - start;
}
