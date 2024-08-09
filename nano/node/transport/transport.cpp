#include "nano/lib/rsnanoutils.hpp"

#include <nano/lib/rsnano.hpp>
#include <nano/node/common.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/transport.hpp>

#include <boost/asio/ip/address.hpp>
#include <boost/asio/ip/address_v4.hpp>
#include <boost/asio/ip/address_v6.hpp>
#include <boost/format.hpp>

nano::endpoint nano::transport::map_endpoint_to_v6 (nano::endpoint const & endpoint_a)
{
	auto endpoint_l (endpoint_a);
	if (endpoint_l.address ().is_v4 ())
	{
		endpoint_l = nano::endpoint (boost::asio::ip::address_v6::v4_mapped (endpoint_l.address ().to_v4 ()), endpoint_l.port ());
	}
	return endpoint_l;
}

nano::endpoint nano::transport::map_tcp_to_endpoint (nano::tcp_endpoint const & endpoint_a)
{
	return nano::endpoint (endpoint_a.address (), endpoint_a.port ());
}

nano::tcp_endpoint nano::transport::map_endpoint_to_tcp (nano::endpoint const & endpoint_a)
{
	return nano::tcp_endpoint (endpoint_a.address (), endpoint_a.port ());
}

boost::asio::ip::address nano::transport::map_address_to_subnetwork (boost::asio::ip::address const & address_a)
{
	debug_assert (address_a.is_v6 ());
	auto octets = address_a.to_v6 ().to_bytes ();
	std::array<uint8_t, 16> result;
	rsnano::rsn_map_address_to_subnetwork (octets.data (), result.data ());
	return boost::asio::ip::address_v6{ result };
}

boost::asio::ip::address nano::transport::ipv4_address_or_ipv6_subnet (boost::asio::ip::address const & address_a)
{
	debug_assert (address_a.is_v6 ());
	auto octets = address_a.to_v6 ().to_bytes ();
	std::array<uint8_t, 16> result;
	rsnano::rsn_ipv4_address_or_ipv6_subnet (octets.data (), result.data ());
	return boost::asio::ip::address_v6{ result };
}

boost::asio::ip::address_v6 nano::transport::mapped_from_v4_bytes (unsigned long address_a)
{
	return boost::asio::ip::address_v6::v4_mapped (boost::asio::ip::address_v4 (address_a));
}

boost::asio::ip::address_v6 nano::transport::mapped_from_v4_or_v6 (boost::asio::ip::address const & address_a)
{
	return address_a.is_v4 () ? boost::asio::ip::address_v6::v4_mapped (address_a.to_v4 ()) : address_a.to_v6 ();
}

bool nano::transport::is_ipv4_or_v4_mapped_address (boost::asio::ip::address const & address_a)
{
	return address_a.is_v4 () || address_a.to_v6 ().is_v4_mapped ();
}

bool nano::transport::reserved_address (nano::endpoint const & endpoint_a, bool allow_local_peers)
{
	debug_assert (endpoint_a.address ().is_v6 ());
	auto bytes (endpoint_a.address ().to_v6 ());
	auto endpoint_dto{ rsnano::udp_endpoint_to_dto (endpoint_a) };
	return rsnano::rsn_reserved_address (&endpoint_dto, allow_local_peers);
}
