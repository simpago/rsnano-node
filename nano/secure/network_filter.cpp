#include <nano/lib/locks.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/secure/buffer.hpp>
#include <nano/secure/common.hpp>
#include <nano/secure/network_filter.hpp>

nano::network_filter::network_filter (size_t size_a) :
	handle (rsnano::rsn_network_filter_create (size_a))
{
}

nano::network_filter::network_filter (rsnano::NetworkFilterHandle * handle_a) :
	handle{ handle_a }
{
}

nano::network_filter::~network_filter ()
{
	rsnano::rsn_network_filter_destroy (handle);
}

bool nano::network_filter::apply (uint8_t const * bytes_a, size_t count_a, nano::uint128_t * digest_a)
{
	std::uint8_t digest_bytes[16];
	auto existed = rsnano::rsn_network_filter_apply (handle, bytes_a, count_a, &digest_bytes[0]);
	if (digest_a != nullptr)
	{
		boost::multiprecision::import_bits (*digest_a, std::begin (digest_bytes), std::end (digest_bytes), 8, true);
	}
	return existed;
}

void nano::network_filter::clear (nano::uint128_t const & digest_a)
{
	std::uint8_t digest_bytes[16];
	boost::multiprecision::export_bits (digest_a, std::begin (digest_bytes), 8, true);
	rsnano::rsn_network_filter_clear (handle, &digest_bytes);
}

void nano::network_filter::clear (std::vector<nano::uint128_t> const & digests_a)
{
	auto digest_bytes = new std::uint8_t[digests_a.size ()][16];
	auto i = 0;
	for (auto d : digests_a)
	{
		boost::multiprecision::export_bits (d, digest_bytes[i], 8, true);
		++i;
	}

	rsnano::rsn_network_filter_clear_many (handle, digest_bytes, digests_a.size ());
	delete[] digest_bytes;
}

void nano::network_filter::clear (uint8_t const * bytes_a, size_t count_a)
{
	rsnano::rsn_network_filter_clear_bytes (handle, bytes_a, count_a);
}

template <typename OBJECT>
void nano::network_filter::clear (OBJECT const & object_a)
{
	clear (hash (object_a));
}

void nano::network_filter::clear ()
{
	rsnano::rsn_network_filter_clear_all (handle);
}

template <typename OBJECT>
nano::uint128_t nano::network_filter::hash (OBJECT const & object_a) const
{
	std::vector<uint8_t> bytes;
	{
		nano::vectorstream stream (bytes);
		object_a->serialize (stream);
	}

	std::uint8_t digest[16];
	rsnano::rsn_network_filter_hash (handle, bytes.data (), bytes.size (), &digest);
	nano::uint128_t result;
	boost::multiprecision::import_bits (result, std::begin (digest), std::end (digest), 8, true);
	return result;
}

// Explicitly instantiate
template nano::uint128_t nano::network_filter::hash (std::shared_ptr<nano::block> const &) const;
template void nano::network_filter::clear (std::shared_ptr<nano::block> const &);
