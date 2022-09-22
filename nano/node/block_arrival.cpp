#include <nano/node/block_arrival.hpp>

nano::block_arrival::block_arrival () :
	handle{ rsnano::rsn_block_arrival_create () }
{
}

nano::block_arrival::~block_arrival ()
{
	rsnano::rsn_block_arrival_destroy (handle);
}

bool nano::block_arrival::add (nano::block_hash const & hash_a)
{
	return rsnano::rsn_block_arrival_add (handle, hash_a.bytes.data ());
}

bool nano::block_arrival::recent (nano::block_hash const & hash_a)
{
	return rsnano::rsn_block_arrival_recent (handle, hash_a.bytes.data ());
}

std::size_t nano::block_arrival::size ()
{
	return rsnano::rsn_block_arrival_size (handle);
}

std::size_t nano::block_arrival::size_of_element () const
{
	return rsnano::rsn_block_arrival_size_of_element (handle);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (block_arrival & block_arrival, std::string const & name)
{
	std::size_t count = block_arrival.size ();
	auto sizeof_element = block_arrival.size_of_element ();
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "arrival", count, sizeof_element }));
	return composite;
}
