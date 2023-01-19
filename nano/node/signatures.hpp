#pragma once

#include <nano/lib/rsnano.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/utility.hpp>

#include <atomic>
#include <future>

namespace nano
{
class signature_check_set final
{
public:
	signature_check_set (std::size_t size, unsigned char const ** messages, std::size_t * message_lengths, unsigned char const ** pub_keys, unsigned char const ** signatures, int * verifications) :
		size (size), messages (messages), message_lengths (message_lengths), pub_keys (pub_keys), signatures (signatures), verifications (verifications)
	{
	}
	std::size_t size;
	unsigned char const ** messages;
	std::size_t * message_lengths;
	unsigned char const ** pub_keys;
	unsigned char const ** signatures;
	int * verifications;
};

/** Multi-threaded signature checker */
class signature_checker final
{
public:
	signature_checker (unsigned num_threads);
	signature_checker (signature_checker const &) = delete;
	~signature_checker ();
	void verify (signature_check_set &);
	void stop ();
	void flush ();
	rsnano::SignatureCheckerHandle const * get_handle () const;

	static std::size_t get_batch_size ();

	signature_checker & operator= (signature_checker const &) = delete;

private:
	rsnano::SignatureCheckerHandle * handle;
};
}
