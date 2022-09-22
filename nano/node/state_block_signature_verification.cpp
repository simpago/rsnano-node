#include <nano/lib/logger_mt.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/timer.hpp>
#include <nano/node/nodeconfig.hpp>
#include <nano/node/signatures.hpp>
#include <nano/node/state_block_signature_verification.hpp>
#include <nano/secure/common.hpp>

#include <boost/format.hpp>

void item_to_dto (nano::state_block_signature_verification::value_type const & value, rsnano::StateBlockSignatureVerificationValueDto & result)
{
	auto const & [block, account, verification] = value;
	result.block = block->clone_handle ();
	std::copy (std::begin (account.bytes), std::end (account.bytes), std::begin (result.account));
	result.verification = static_cast<uint8_t> (verification);
}

void dto_to_value_type (rsnano::StateBlockSignatureVerificationValueDto const & dto, nano::state_block_signature_verification::value_type & result)
{
	nano::account account;
	std::copy (std::begin (dto.account), std::end (dto.account), std::begin (account.bytes));
	auto verification = static_cast<nano::signature_verification> (dto.verification);
	result = nano::state_block_signature_verification::value_type (nano::block_handle_to_block (dto.block), account, verification);
}

void blocks_verified_callback_adapter (void * context, const rsnano::StateBlockSignatureVerificationResultDto * result_dto)
{
	std::vector<int> verifications (result_dto->verifications, result_dto->verifications + result_dto->size);
	std::vector<nano::block_hash> hashes;
	for (auto i = result_dto->hashes; i != result_dto->hashes + result_dto->size; ++i)
	{
		nano::block_hash hash;
		std::copy (std::begin (*i), std::end (*i), std::begin (hash.bytes));
		hashes.push_back (hash);
	}

	std::vector<nano::signature> blocks_signatures;
	blocks_signatures.reserve (result_dto->size);
	for (auto i = result_dto->signatures; i != result_dto->signatures + result_dto->size; ++i)
	{
		nano::signature signature;
		std::copy (std::begin (*i), std::end (*i), std::begin (signature.bytes));
		blocks_signatures.push_back (signature);
	}

	std::deque<nano::state_block_signature_verification::value_type> items;
	for (auto i = result_dto->items; i != result_dto->items + result_dto->size; i++)
	{
		nano::state_block_signature_verification::value_type value;
		dto_to_value_type (*i, value);
		items.push_back (value);
	}

	auto instance = reinterpret_cast<nano::state_block_signature_verification *> (context);
	instance->blocks_verified_callback (items, verifications, hashes, blocks_signatures);
}

void transition_inactive_callback_adapter (void * context)
{
	auto instance = reinterpret_cast<nano::state_block_signature_verification *> (context);
	instance->transition_inactive_callback ();
}

nano::state_block_signature_verification::state_block_signature_verification (nano::signature_checker & signature_checker, nano::epochs & epochs, bool timing_logging, std::shared_ptr<nano::logger_mt> & logger, uint64_t state_block_signature_verification_size)
{
	handle = rsnano::rsn_state_block_signature_verification_create (signature_checker.get_handle (), epochs.get_handle (), nano::to_logger_handle (logger), timing_logging, state_block_signature_verification_size);
	rsnano::rsn_state_block_signature_verification_verified_callback (handle, blocks_verified_callback_adapter, this);
	rsnano::rsn_state_block_signature_verification_transition_inactive_callback (handle, transition_inactive_callback_adapter, this);
}

nano::state_block_signature_verification::~state_block_signature_verification ()
{
	rsnano::rsn_state_block_signature_verification_destroy (handle);
}

void nano::state_block_signature_verification::stop ()
{
	rsnano::rsn_state_block_signature_verification_stop (handle);
}

bool nano::state_block_signature_verification::is_active ()
{
	return rsnano::rsn_state_block_signature_verification_is_active (handle);
}

void nano::state_block_signature_verification::add (value_type const & item)
{
	rsnano::StateBlockSignatureVerificationValueDto dto;
	item_to_dto (item, dto);
	rsnano::rsn_state_block_signature_verification_add (handle, &dto);
	rsnano::rsn_block_destroy (dto.block);
}

std::size_t nano::state_block_signature_verification::size ()
{
	return rsnano::rsn_state_block_signature_verification_size (handle);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (state_block_signature_verification & state_block_signature_verification, std::string const & name)
{
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "state_blocks", state_block_signature_verification.size (), sizeof (state_block_signature_verification::value_type) }));
	return composite;
}