#include <nano/lib/rep_weights.hpp>
#include <nano/secure/store.hpp>

nano::rep_weights::rep_weights () :
	handle{ rsnano::rsn_rep_weights_create () }
{
}

nano::rep_weights::rep_weights (rsnano::RepWeightsHandle * handle_a) :
	handle{ handle_a } {};

nano::rep_weights::rep_weights (nano::rep_weights && other_a) :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
};

nano::rep_weights::~rep_weights ()
{
	if (handle != nullptr)
		rsnano::rsn_rep_weights_destroy (handle);
}

nano::rep_weights & nano::rep_weights::operator= (nano::rep_weights && other_a)
{
	if (handle != nullptr)
		rsnano::rsn_rep_weights_destroy (handle);
	handle = other_a.handle;
	other_a.handle = nullptr;
	return *this;
}

void nano::rep_weights::representation_add (nano::account const & source_rep_a, nano::uint128_t const & amount_a)
{
	std::uint8_t amount_bytes[16] = { 0 };
	boost::multiprecision::export_bits (amount_a, std::rbegin (amount_bytes), 8, false);
	rsnano::rsn_rep_weights_representation_add (handle, source_rep_a.bytes.data (), &amount_bytes[0]);
}

void nano::rep_weights::representation_add_dual (nano::account const & source_rep_1, nano::uint128_t const & amount_1, nano::account const & source_rep_2, nano::uint128_t const & amount_2)
{
	std::uint8_t amount_1_bytes[16] = { 0 };
	std::uint8_t amount_2_bytes[16] = { 0 };
	boost::multiprecision::export_bits (amount_1, std::rbegin (amount_1_bytes), 8, false);
	boost::multiprecision::export_bits (amount_2, std::rbegin (amount_2_bytes), 8, false);
	rsnano::rsn_rep_weights_representation_add_dual (handle, source_rep_1.bytes.data (), &amount_1_bytes[0], source_rep_2.bytes.data (), &amount_2_bytes[0]);
}

nano::uint128_t nano::rep_weights::representation_get (nano::account const & account_a) const
{
	uint8_t representation[16];
	rsnano::rsn_rep_weights_representation_get (handle, account_a.bytes.data (), &representation[0]);
	nano::uint128_t result;
	boost::multiprecision::import_bits (result, std::begin (representation), std::end (representation), 8, true);
	return result;
}

/** Makes a copy */
std::unordered_map<nano::account, nano::uint128_t> nano::rep_weights::get_rep_amounts () const
{
	rsnano::RepAmountsDto amounts_dto;
	rsnano::rsn_rep_weights_get_rep_amounts (handle, &amounts_dto);
	std::unordered_map<nano::account, nano::uint128_t> result;
	rsnano::RepAmountItemDto const * current;
	int i;
	for (i = 0, current = amounts_dto.items; i < amounts_dto.count; ++i)
	{
		nano::account account;
		nano::uint128_t amount;
		std::copy (std::begin (current->account), std::end (current->account), std::begin (account.bytes));
		boost::multiprecision::import_bits (amount, std::begin (current->amount), std::end (current->amount), 8, true);
		result.insert ({ account, amount });
		current++;
	}

	rsnano::rsn_rep_weights_destroy_amounts_dto (&amounts_dto);

	return result;
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (nano::rep_weights const & rep_weights, std::string const & name)
{
	size_t rep_amounts_count = rsnano::rsn_rep_weights_item_count (rep_weights.handle);
	auto sizeof_element = rsnano::rsn_rep_weights_item_size ();
	auto composite = std::make_unique<nano::container_info_composite> (name);
	composite->add_component (std::make_unique<nano::container_info_leaf> (container_info{ "rep_amounts", rep_amounts_count, sizeof_element }));
	return composite;
}
