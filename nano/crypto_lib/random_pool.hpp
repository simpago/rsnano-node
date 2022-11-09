#pragma once

#include <mutex>

namespace nano
{
class random_pool
{
public:
	static void generate_block (unsigned char * output, size_t size);
	static unsigned generate_word32 (unsigned min, unsigned max);
	static unsigned char generate_byte ();

	random_pool () = delete;
	random_pool (random_pool const &) = delete;
	random_pool & operator= (random_pool const &) = delete;

private:
	template <class Iter>
	friend void random_pool_shuffle (Iter begin, Iter end);
};
}
