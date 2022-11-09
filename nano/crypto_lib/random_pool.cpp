#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/rsnano.hpp>

void nano::random_pool::generate_block (unsigned char * output, size_t size)
{
	rsnano::rsn_random_pool_generate_block (output, size);
}

unsigned nano::random_pool::generate_word32 (unsigned min, unsigned max)
{
	return rsnano::rsn_random_pool_generate_word32 (min, max);
}

unsigned char nano::random_pool::generate_byte ()
{
	return rsnano::rsn_random_pool_generate_byte ();
}