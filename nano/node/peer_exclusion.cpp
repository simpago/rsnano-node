#include "nano/lib/rsnanoutils.hpp"

#include <nano/node/common.hpp>
#include <nano/node/peer_exclusion.hpp>

nano::peer_exclusion::peer_exclusion () :
	handle{ rsnano::rsn_peer_exclusion_create () }
{
}

nano::peer_exclusion::~peer_exclusion ()
{
	rsnano::rsn_peer_exclusion_destroy (handle);
}

uint64_t nano::peer_exclusion::add (nano::tcp_endpoint const & endpoint_a, std::size_t const network_peers_count_a)
{
	auto endpoint_dto{ rsnano::endpoint_to_dto (endpoint_a) };
	return rsnano::rsn_peer_exclusion_add (handle, &endpoint_dto, network_peers_count_a);
}

bool nano::peer_exclusion::check (nano::tcp_endpoint const & endpoint_a)
{
	auto endpoint_dto{ rsnano::endpoint_to_dto (endpoint_a) };
	return rsnano::rsn_peer_exclusion_check (handle, &endpoint_dto);
}

void nano::peer_exclusion::remove (nano::tcp_endpoint const & endpoint_a)
{
	auto endpoint_dto{ rsnano::endpoint_to_dto (endpoint_a) };
	rsnano::rsn_peer_exclusion_remove (handle, &endpoint_dto);
}

std::size_t nano::peer_exclusion::size () const
{
	return rsnano::rsn_peer_exclusion_size (handle);
}

bool nano::peer_exclusion::contains (const nano::tcp_endpoint & endpoint_a)
{
	auto endpoint_dto{ rsnano::endpoint_to_dto (endpoint_a) };
	return rsnano::rsn_peer_exclusion_contains (handle, &endpoint_dto);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (nano::peer_exclusion const & excluded_peers, std::string const & name)
{
	auto composite = std::make_unique<container_info_composite> (name);

	std::size_t excluded_peers_count = excluded_peers.size ();
	auto sizeof_excluded_peers_element = rsnano::rsn_peer_exclusion_element_size ();
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "peers", excluded_peers_count, sizeof_excluded_peers_element }));

	return composite;
}
