#include <nano/lib/blocks.hpp>
#include <nano/lib/memory.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/node/active_transactions.hpp>
#include <nano/node/common.hpp>
#include <nano/node/election.hpp>
#include <nano/node/network.hpp>
#include <nano/node/wallet.hpp>
#include <nano/secure/buffer.hpp>

#include <boost/format.hpp>

uint64_t nano::ip_address_hash_raw (boost::asio::ip::address const & ip_a, uint16_t port)
{
	debug_assert (ip_a.is_v6 ());
	return rsnano::rsn_ip_address_hash_raw (ip_a.to_v6 ().to_bytes ().data (), port);
}

bool nano::parse_port (std::string const & string_a, uint16_t & port_a)
{
	bool result = false;
	try
	{
		port_a = boost::lexical_cast<uint16_t> (string_a);
	}
	catch (...)
	{
		result = true;
	}
	return result;
}

// Can handle both ipv4 & ipv6 addresses (with and without square brackets)
bool nano::parse_address (std::string const & address_text_a, boost::asio::ip::address & address_a)
{
	auto address_text = address_text_a;
	if (!address_text.empty () && address_text.front () == '[' && address_text.back () == ']')
	{
		// Chop the square brackets off as make_address doesn't always like them
		address_text = address_text.substr (1, address_text.size () - 2);
	}

	boost::system::error_code address_ec;
	address_a = boost::asio::ip::make_address (address_text, address_ec);
	return !!address_ec;
}

bool nano::parse_address_port (std::string const & string, boost::asio::ip::address & address_a, uint16_t & port_a)
{
	auto result (false);
	auto port_position (string.rfind (':'));
	if (port_position != std::string::npos && port_position > 0)
	{
		std::string port_string (string.substr (port_position + 1));
		try
		{
			uint16_t port;
			result = parse_port (port_string, port);
			if (!result)
			{
				boost::system::error_code ec;
				auto address (boost::asio::ip::make_address_v6 (string.substr (0, port_position), ec));
				if (!ec)
				{
					address_a = address;
					port_a = port;
				}
				else
				{
					result = true;
				}
			}
			else
			{
				result = true;
			}
		}
		catch (...)
		{
			result = true;
		}
	}
	else
	{
		result = true;
	}
	return result;
}

bool nano::parse_endpoint (std::string const & string, nano::endpoint & endpoint_a)
{
	boost::asio::ip::address address;
	uint16_t port;
	auto result (parse_address_port (string, address, port));
	if (!result)
	{
		endpoint_a = nano::endpoint (address, port);
	}
	return result;
}

bool nano::parse_tcp_endpoint (std::string const & string, nano::tcp_endpoint & endpoint_a)
{
	boost::asio::ip::address address;
	uint16_t port;
	auto result (parse_address_port (string, address, port));
	if (!result)
	{
		endpoint_a = nano::tcp_endpoint (address, port);
	}
	return result;
}

std::chrono::seconds nano::telemetry_cache_cutoffs::dev ()
{
	return std::chrono::seconds (rsnano::rsn_telemetry_cache_cutoffs_dev ());
}

std::chrono::seconds nano::telemetry_cache_cutoffs::network_to_time (network_constants const & network_constants)
{
	auto dto = network_constants.to_dto ();
	return std::chrono::seconds{ rsnano::rsn_telemetry_cache_cutoffs_network_to_time_s (&dto) };
}

nano::node_singleton_memory_pool_purge_guard::node_singleton_memory_pool_purge_guard () :
	cleanup_guard ({ nano::block_memory_pool_purge, nano::purge_shared_ptr_singleton_pool_memory<nano::vote>, nano::purge_shared_ptr_singleton_pool_memory<nano::election> })
{
}
