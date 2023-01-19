#include <nano/boost/asio/post.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/node/signatures.hpp>

rsnano::SignatureCheckSetDto to_check_set_dto (nano::signature_check_set const & check_a)
{
	rsnano::SignatureCheckSetDto check_dto;
	check_dto.messages = check_a.messages;
	check_dto.message_lengths = check_a.message_lengths;
	check_dto.pub_keys = check_a.pub_keys;
	check_dto.signatures = check_a.signatures;
	check_dto.verifications = check_a.verifications;
	check_dto.size = check_a.size;
	return check_dto;
}

nano::signature_checker::signature_checker (unsigned num_threads) :
	handle (rsnano::rsn_signature_checker_create (num_threads))
{
}

nano::signature_checker::~signature_checker ()
{
	rsnano::rsn_signature_checker_destroy (handle);
}

std::size_t nano::signature_checker::get_batch_size ()
{
	return rsnano::rsn_signature_checker_batch_size ();
}

void nano::signature_checker::verify (nano::signature_check_set & check_a)
{
	rsnano::SignatureCheckSetDto check_set_dto{ to_check_set_dto (check_a) };
	rsnano::rsn_signature_checker_verify (handle, &check_set_dto);
}

void nano::signature_checker::stop ()
{
	rsnano::rsn_signature_checker_stop (handle);
}

void nano::signature_checker::flush ()
{
	bool is_ok = rsnano::rsn_signature_checker_flush (handle);
	release_assert (is_ok && "timeout in flush");
}

rsnano::SignatureCheckerHandle const * nano::signature_checker::get_handle () const
{
	return handle;
}