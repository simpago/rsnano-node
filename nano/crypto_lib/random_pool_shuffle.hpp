#pragma once

#include <nano/crypto_lib/random_pool.hpp>

namespace nano
{
template <class Iter>
void random_pool_shuffle (Iter begin, Iter end)
{
	for (; begin != end; ++begin)
		std::iter_swap (begin, begin + random_pool::generate_word32 (0, static_cast<uint32_t> (end - begin - 1)));
}
}
