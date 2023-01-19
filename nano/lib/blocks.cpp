#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/lib/memory.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/threading.hpp>

#include <boost/property_tree/json_parser.hpp>

#include <bitset>

/** Compare blocks, first by type, then content. This is an optimization over dynamic_cast, which is very slow on some platforms. */
namespace
{
template <typename T>
bool blocks_equal (T const & first, nano::block const & second)
{
	static_assert (std::is_base_of<nano::block, T>::value, "Input parameter is not a block type");
	return (first.type () == second.type ()) && (static_cast<T const &> (second)) == first;
}
}

void nano::block_memory_pool_purge ()
{
	nano::purge_shared_ptr_singleton_pool_memory<nano::open_block> ();
	nano::purge_shared_ptr_singleton_pool_memory<nano::state_block> ();
	nano::purge_shared_ptr_singleton_pool_memory<nano::send_block> ();
	nano::purge_shared_ptr_singleton_pool_memory<nano::change_block> ();
}

nano::block::~block ()
{
	if (handle != nullptr)
	{
		rsnano::rsn_block_destroy (handle);
	}
}

std::string nano::block::to_json () const
{
	std::string result;
	serialize_json (result);
	return result;
}

size_t nano::block::size (nano::block_type type_a)
{
	return rsnano::rsn_block_serialized_size (static_cast<uint8_t> (type_a));
}

nano::work_version nano::block::work_version () const
{
	return nano::work_version::work_1;
}

void nano::block::refresh ()
{
	if (!cached_hash.is_zero ())
	{
		cached_hash = generate_hash ();
	}
}

uint64_t nano::block::block_work () const
{
	return rsnano::rsn_block_work (handle);
}

void nano::block::block_work_set (uint64_t work_a)
{
	rsnano::rsn_block_work_set (handle, work_a);
}

nano::block_type nano::block::type () const
{
	return static_cast<nano::block_type> (rsnano::rsn_block_type (handle));
}

nano::block_hash const & nano::block::hash () const
{
	if (!cached_hash.is_zero ())
	{
		// Once a block is created, it should not be modified (unless using refresh ())
		// This would invalidate the cache; check it hasn't changed.
		debug_assert (cached_hash == generate_hash ());
	}
	else
	{
		cached_hash = generate_hash ();
	}

	return cached_hash;
}

nano::block_hash nano::block::full_hash () const
{
	nano::block_hash result;
	rsnano::rsn_block_full_hash (handle, result.bytes.data ());
	return result;
}

nano::block_sideband nano::block::sideband () const
{
	rsnano::BlockSidebandDto dto;
	auto result = rsnano::rsn_block_sideband (get_handle (), &dto);
	debug_assert (result == 0);
	return nano::block_sideband (dto);
}

void nano::block::sideband_set (nano::block_sideband const & sideband_a)
{
	if (rsnano::rsn_block_sideband_set (get_handle (), &sideband_a.as_dto ()) < 0)
		throw std::runtime_error ("cannot set sideband");
}

bool nano::block::has_sideband () const
{
	return rsnano::rsn_block_has_sideband (get_handle ());
}

rsnano::BlockHandle * nano::block::get_handle () const
{
	return handle;
}

nano::account nano::block::representative () const
{
	static nano::account representative{};
	return representative;
}

nano::block_hash nano::block::source () const
{
	static nano::block_hash source{ 0 };
	return source;
}

nano::account nano::block::destination () const
{
	static nano::account destination{};
	return destination;
}

nano::link nano::block::link () const
{
	static nano::link link{ 0 };
	return link;
}

nano::account nano::block::account () const
{
	static nano::account account{};
	return account;
}

rsnano::BlockHandle * nano::block::clone_handle () const
{
	return rsnano::rsn_block_handle_clone (handle);
}

const void * nano::block::get_rust_data_pointer () const
{
	return rsnano::rsn_block_rust_data_pointer (handle);
}

nano::qualified_root nano::block::qualified_root () const
{
	return nano::qualified_root (root (), previous ());
}

nano::amount nano::block::balance () const
{
	static nano::amount amount{ 0 };
	return amount;
}

void nano::block::sign_zero ()
{
	signature_set (nano::signature (0));
}

void nano::block::serialize (nano::stream & stream_a) const
{
	if (rsnano::rsn_block_serialize (handle, &stream_a) != 0)
	{
		throw std::runtime_error ("could not serialize block");
	}
}

nano::block_hash nano::block::previous () const
{
	uint8_t buffer[32];
	rsnano::rsn_block_previous (handle, &buffer);
	nano::block_hash result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

nano::signature nano::block::block_signature () const
{
	uint8_t bytes[64];
	rsnano::rsn_block_signature (handle, &bytes);
	nano::signature result;
	std::copy (std::begin (bytes), std::end (bytes), std::begin (result.bytes));
	return result;
}

void nano::block::signature_set (nano::signature const & signature_a)
{
	uint8_t bytes[64];
	std::copy (std::begin (signature_a.bytes), std::end (signature_a.bytes), std::begin (bytes));
	rsnano::rsn_block_signature_set (handle, &bytes);
}

void nano::block::serialize_json (std::string & string_a, bool single_line) const
{
	boost::property_tree::ptree tree;
	serialize_json (tree);
	std::stringstream ostream;
	boost::property_tree::write_json (ostream, tree, !single_line);
	string_a = ostream.str ();
}

void nano::block::serialize_json (boost::property_tree::ptree & tree) const
{
	if (rsnano::rsn_block_serialize_json (handle, &tree) < 0)
		throw std::runtime_error ("could not serialize send_block as JSON");
}

nano::block_hash nano::block::generate_hash () const
{
	uint8_t bytes[32];
	rsnano::rsn_block_hash (handle, &bytes);
	nano::block_hash result;
	std::copy (std::begin (bytes), std::end (bytes), std::begin (result.bytes));
	return result;
}

void nano::send_block::visit (nano::block_visitor & visitor_a) const
{
	visitor_a.send_block (*this);
}

void nano::send_block::visit (nano::mutable_block_visitor & visitor_a)
{
	visitor_a.send_block (*this);
}

void nano::send_block::zero ()
{
	rsnano::rsn_send_block_zero (handle);
}

void nano::send_block::set_destination (nano::account account_a)
{
	uint8_t bytes[32];
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (bytes));
	rsnano::rsn_send_block_destination_set (handle, &bytes);
}

void nano::send_block::previous_set (nano::block_hash previous_a)
{
	uint8_t bytes[32];
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (bytes));
	rsnano::rsn_send_block_previous_set (handle, &bytes);
}

void nano::send_block::balance_set (nano::amount balance_a)
{
	uint8_t bytes[16];
	std::copy (std::begin (balance_a.bytes), std::end (balance_a.bytes), std::begin (bytes));
	rsnano::rsn_send_block_balance_set (handle, &bytes);
}

nano::send_block::send_block (nano::block_hash const & previous_a, nano::account const & destination_a, nano::amount const & balance_a, nano::raw_key const & prv_a, nano::public_key const & pub_a, uint64_t work_a)
{
	debug_assert (destination_a != nullptr);
	debug_assert (pub_a != nullptr);

	rsnano::SendBlockDto2 dto;
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (dto.previous));
	std::copy (std::begin (destination_a.bytes), std::end (destination_a.bytes), std::begin (dto.destination));
	std::copy (std::begin (balance_a.bytes), std::end (balance_a.bytes), std::begin (dto.balance));
	std::copy (std::begin (prv_a.bytes), std::end (prv_a.bytes), std::begin (dto.priv_key));
	std::copy (std::begin (pub_a.bytes), std::end (pub_a.bytes), std::begin (dto.pub_key));
	dto.work = work_a;
	handle = rsnano::rsn_send_block_create2 (&dto);
	if (handle == nullptr)
		throw std::runtime_error ("could not create send_block");
}

nano::send_block::send_block (bool & error_a, nano::stream & stream_a)
{
	handle = rsnano::rsn_send_block_deserialize (&stream_a);
	error_a = handle == nullptr;
}

nano::send_block::send_block (bool & error_a, boost::property_tree::ptree const & tree_a)
{
	handle = rsnano::rsn_send_block_deserialize_json (&tree_a);
	error_a = handle == nullptr;
}

rsnano::SendBlockDto empty_send_block_dto ()
{
	rsnano::SendBlockDto dto;
	std::fill (std::begin (dto.signature), std::end (dto.signature), 0);
	std::fill (std::begin (dto.previous), std::end (dto.previous), 0);
	std::fill (std::begin (dto.destination), std::end (dto.destination), 0);
	std::fill (std::begin (dto.balance), std::end (dto.balance), 0);
	dto.work = 0;
	return dto;
}

nano::send_block::send_block ()
{
	auto dto{ empty_send_block_dto () };
	handle = rsnano::rsn_send_block_create (&dto);
}

nano::send_block::send_block (const send_block & other)
{
	cached_hash = other.cached_hash;
	if (other.handle == nullptr)
	{
		handle = nullptr;
	}
	else
	{
		handle = rsnano::rsn_block_clone (other.handle);
	}
}

nano::send_block::send_block (send_block && other)
{
	cached_hash = other.cached_hash;
	handle = other.handle;
	other.handle = nullptr;
}

nano::send_block::send_block (rsnano::BlockHandle * handle_a)
{
	handle = handle_a;
}

bool nano::send_block::operator== (nano::block const & other_a) const
{
	return blocks_equal (*this, other_a);
}

bool nano::send_block::valid_predecessor (nano::block const & block_a) const
{
	return rsnano::rsn_send_block_valid_predecessor (static_cast<uint8_t> (block_a.type ()));
}

bool nano::send_block::operator== (nano::send_block const & other_a) const
{
	return rsnano::rsn_block_equals (handle, other_a.handle);
}

nano::account nano::send_block::destination () const
{
	uint8_t buffer[32];
	rsnano::rsn_send_block_destination (handle, &buffer);
	nano::account result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

nano::root nano::send_block::root () const
{
	return previous ();
}

nano::amount nano::send_block::balance () const
{
	uint8_t buffer[16];
	rsnano::rsn_send_block_balance (handle, &buffer);
	nano::amount result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

std::size_t nano::send_block::size ()
{
	return sizeof (nano::block_hash) + sizeof (nano::account) + sizeof (nano::amount) + sizeof (nano::signature) + sizeof (uint64_t);
}

nano::open_block::open_block ()
{
	rsnano::OpenBlockDto dto;
	dto.work = 0;
	std::fill (std::begin (dto.account), std::end (dto.account), 0);
	std::fill (std::begin (dto.source), std::end (dto.source), 0);
	std::fill (std::begin (dto.representative), std::end (dto.representative), 0);
	std::fill (std::begin (dto.signature), std::end (dto.signature), 0);
	handle = rsnano::rsn_open_block_create (&dto);
}

nano::open_block::open_block (nano::block_hash const & source_a, nano::account const & representative_a, nano::account const & account_a, nano::raw_key const & prv_a, nano::public_key const & pub_a, uint64_t work_a)
{
	debug_assert (representative_a != nullptr);
	debug_assert (account_a != nullptr);
	debug_assert (pub_a != nullptr);

	rsnano::OpenBlockDto2 dto;
	std::copy (std::begin (source_a.bytes), std::end (source_a.bytes), std::begin (dto.source));
	std::copy (std::begin (representative_a.bytes), std::end (representative_a.bytes), std::begin (dto.representative));
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (dto.account));
	std::copy (std::begin (prv_a.bytes), std::end (prv_a.bytes), std::begin (dto.priv_key));
	std::copy (std::begin (pub_a.bytes), std::end (pub_a.bytes), std::begin (dto.pub_key));
	dto.work = work_a;
	handle = rsnano::rsn_open_block_create2 (&dto);
	if (handle == nullptr)
		throw std::runtime_error ("could not create open_block");
}

nano::open_block::open_block (nano::block_hash const & source_a, nano::account const & representative_a, nano::account const & account_a, std::nullptr_t)
{
	debug_assert (representative_a != nullptr);
	debug_assert (account_a != nullptr);

	rsnano::OpenBlockDto dto;
	dto.work = 0;
	std::copy (std::begin (source_a.bytes), std::end (source_a.bytes), std::begin (dto.source));
	std::copy (std::begin (representative_a.bytes), std::end (representative_a.bytes), std::begin (dto.representative));
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (dto.account));
	std::fill (std::begin (dto.signature), std::end (dto.signature), 0);
	handle = rsnano::rsn_open_block_create (&dto);
}

nano::open_block::open_block (bool & error_a, nano::stream & stream_a)
{
	handle = rsnano::rsn_open_block_deserialize (&stream_a);
	error_a = handle == nullptr;
}

nano::open_block::open_block (bool & error_a, boost::property_tree::ptree const & tree_a)
{
	handle = rsnano::rsn_open_block_deserialize_json (&tree_a);
	error_a = handle == nullptr;
}

nano::open_block::open_block (const open_block & other)
{
	cached_hash = other.cached_hash;
	if (other.handle == nullptr)
	{
		handle = nullptr;
	}
	else
	{
		handle = rsnano::rsn_block_clone (other.handle);
	}
}

nano::open_block::open_block (nano::open_block && other)
{
	cached_hash = other.cached_hash;
	handle = other.handle;
	other.handle = nullptr;
}

nano::open_block::open_block (rsnano::BlockHandle * handle_a)
{
	handle = handle_a;
}

nano::account nano::open_block::account () const
{
	uint8_t buffer[32];
	rsnano::rsn_open_block_account (handle, &buffer);
	nano::account result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

void nano::open_block::visit (nano::block_visitor & visitor_a) const
{
	visitor_a.open_block (*this);
}

void nano::open_block::visit (nano::mutable_block_visitor & visitor_a)
{
	visitor_a.open_block (*this);
}

bool nano::open_block::operator== (nano::block const & other_a) const
{
	return blocks_equal (*this, other_a);
}

bool nano::open_block::operator== (nano::open_block const & other_a) const
{
	return rsnano::rsn_block_equals (handle, other_a.handle);
}

bool nano::open_block::valid_predecessor (nano::block const & block_a) const
{
	return false;
}

nano::block_hash nano::open_block::source () const
{
	uint8_t buffer[32];
	rsnano::rsn_open_block_source (handle, &buffer);
	nano::block_hash result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

nano::root nano::open_block::root () const
{
	return account ();
}

nano::account nano::open_block::representative () const
{
	uint8_t buffer[32];
	rsnano::rsn_open_block_representative (handle, &buffer);
	nano::account result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

void nano::open_block::source_set (nano::block_hash source_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (source_a.bytes), std::end (source_a.bytes), std::begin (buffer));
	rsnano::rsn_open_block_source_set (handle, &buffer);
}

void nano::open_block::account_set (nano::account account_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (buffer));
	rsnano::rsn_open_block_account_set (handle, &buffer);
}

void nano::open_block::representative_set (nano::account account_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (buffer));
	rsnano::rsn_open_block_representative_set (handle, &buffer);
}

void nano::open_block::zero ()
{
	block_work_set (0);
	sign_zero ();
	account_set (0);
	representative_set (0);
	source_set (0);
}

std::size_t nano::open_block::size ()
{
	return rsnano::rsn_open_block_size ();
}

nano::change_block::change_block ()
{
	rsnano::ChangeBlockDto dto;
	std::fill (std::begin (dto.previous), std::end (dto.previous), 0);
	std::fill (std::begin (dto.representative), std::end (dto.representative), 0);
	std::fill (std::begin (dto.signature), std::end (dto.signature), 0);
	dto.work = 0;
	handle = rsnano::rsn_change_block_create (&dto);
}

nano::change_block::change_block (nano::block_hash const & previous_a, nano::account const & representative_a, nano::raw_key const & prv_a, nano::public_key const & pub_a, uint64_t work_a)
{
	debug_assert (representative_a != nullptr);
	debug_assert (pub_a != nullptr);

	rsnano::ChangeBlockDto2 dto;
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (dto.previous));
	std::copy (std::begin (representative_a.bytes), std::end (representative_a.bytes), std::begin (dto.representative));
	std::copy (std::begin (prv_a.bytes), std::end (prv_a.bytes), std::begin (dto.priv_key));
	std::copy (std::begin (pub_a.bytes), std::end (pub_a.bytes), std::begin (dto.pub_key));
	dto.work = work_a;

	handle = rsnano::rsn_change_block_create2 (&dto);
	if (handle == nullptr)
		throw std::runtime_error ("could not create change_block");
}

nano::change_block::change_block (bool & error_a, nano::stream & stream_a)
{
	handle = rsnano::rsn_change_block_deserialize (&stream_a);
	error_a = handle == nullptr;
}

nano::change_block::change_block (bool & error_a, boost::property_tree::ptree const & tree_a)
{
	handle = rsnano::rsn_change_block_deserialize_json (&tree_a);
	error_a = handle == nullptr;
}

nano::change_block::change_block (const nano::change_block & other_a)
{
	cached_hash = other_a.cached_hash;
	if (other_a.handle == nullptr)
	{
		handle = nullptr;
	}
	else
	{
		handle = rsnano::rsn_block_clone (other_a.handle);
	}
}

nano::change_block::change_block (nano::change_block && other_a)
{
	cached_hash = other_a.cached_hash;
	handle = other_a.handle;
	other_a.handle = nullptr;
}

nano::change_block::change_block (rsnano::BlockHandle * handle_a)
{
	handle = handle_a;
}

void nano::change_block::visit (nano::block_visitor & visitor_a) const
{
	visitor_a.change_block (*this);
}

void nano::change_block::visit (nano::mutable_block_visitor & visitor_a)
{
	visitor_a.change_block (*this);
}

bool nano::change_block::operator== (nano::block const & other_a) const
{
	return blocks_equal (*this, other_a);
}

bool nano::change_block::operator== (nano::change_block const & other_a) const
{
	return rsnano::rsn_block_equals (handle, other_a.handle);
}

bool nano::change_block::valid_predecessor (nano::block const & block_a) const
{
	bool result;
	switch (block_a.type ())
	{
		case nano::block_type::send:
		case nano::block_type::receive:
		case nano::block_type::open:
		case nano::block_type::change:
			result = true;
			break;
		default:
			result = false;
			break;
	}
	return result;
}

nano::root nano::change_block::root () const
{
	return previous ();
}

nano::account nano::change_block::representative () const
{
	uint8_t buffer[32];
	rsnano::rsn_change_block_representative (handle, &buffer);
	nano::account result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

void nano::change_block::previous_set (nano::block_hash previous_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (buffer));
	rsnano::rsn_change_block_previous_set (handle, &buffer);
}

void nano::change_block::representative_set (nano::account account_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (buffer));
	rsnano::rsn_change_block_representative_set (handle, &buffer);
}

void nano::change_block::zero ()
{
	block_work_set (0);
	sign_zero ();
	previous_set (0);
	representative_set (0);
}

std::size_t nano::change_block::size ()
{
	return rsnano::rsn_change_block_size ();
}

nano::state_block::state_block ()
{
	rsnano::StateBlockDto dto;
	dto.work = 0;
	std::fill (std::begin (dto.account), std::end (dto.account), 0);
	std::fill (std::begin (dto.previous), std::end (dto.previous), 0);
	std::fill (std::begin (dto.representative), std::end (dto.representative), 0);
	std::fill (std::begin (dto.balance), std::end (dto.balance), 0);
	std::fill (std::begin (dto.link), std::end (dto.link), 0);
	std::fill (std::begin (dto.signature), std::end (dto.signature), 0);
	handle = rsnano::rsn_state_block_create (&dto);
}

nano::state_block::state_block (nano::account const & account_a, nano::block_hash const & previous_a, nano::account const & representative_a, nano::amount const & balance_a, nano::link const & link_a, nano::raw_key const & prv_a, nano::public_key const & pub_a, uint64_t work_a)
{
	debug_assert (account_a != nullptr);
	debug_assert (representative_a != nullptr);
	debug_assert (link_a.as_account () != nullptr);
	debug_assert (pub_a != nullptr);

	rsnano::StateBlockDto2 dto;
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (dto.account));
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (dto.previous));
	std::copy (std::begin (representative_a.bytes), std::end (representative_a.bytes), std::begin (dto.representative));
	std::copy (std::begin (link_a.bytes), std::end (link_a.bytes), std::begin (dto.link));
	std::copy (std::begin (balance_a.bytes), std::end (balance_a.bytes), std::begin (dto.balance));
	std::copy (std::begin (prv_a.bytes), std::end (prv_a.bytes), std::begin (dto.priv_key));
	std::copy (std::begin (pub_a.bytes), std::end (pub_a.bytes), std::begin (dto.pub_key));
	dto.work = work_a;
	handle = rsnano::rsn_state_block_create2 (&dto);
	if (handle == nullptr)
		throw std::runtime_error ("could not create state_block");
}

nano::state_block::state_block (bool & error_a, nano::stream & stream_a)
{
	handle = rsnano::rsn_state_block_deserialize (&stream_a);
	error_a = handle == nullptr;
}

nano::state_block::state_block (bool & error_a, boost::property_tree::ptree const & tree_a)
{
	handle = rsnano::rsn_state_block_deserialize_json (&tree_a);
	error_a = handle == nullptr;
}

nano::state_block::state_block (const nano::state_block & other)
{
	cached_hash = other.cached_hash;
	if (other.handle == nullptr)
	{
		handle = nullptr;
	}
	else
	{
		handle = rsnano::rsn_block_clone (other.handle);
	}
}

nano::state_block::state_block (nano::state_block && other)
{
	cached_hash = other.cached_hash;
	handle = other.handle;
	other.handle = nullptr;
}

nano::state_block::state_block (rsnano::BlockHandle * handle_a)
{
	handle = handle_a;
}

nano::account nano::state_block::account () const
{
	uint8_t buffer[32];
	rsnano::rsn_state_block_account (handle, &buffer);
	nano::account result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

void nano::state_block::visit (nano::block_visitor & visitor_a) const
{
	visitor_a.state_block (*this);
}

void nano::state_block::visit (nano::mutable_block_visitor & visitor_a)
{
	visitor_a.state_block (*this);
}

bool nano::state_block::operator== (nano::block const & other_a) const
{
	return blocks_equal (*this, other_a);
}

bool nano::state_block::operator== (nano::state_block const & other_a) const
{
	return rsnano::rsn_block_equals (handle, other_a.handle);
}

nano::state_block & nano::state_block::operator= (const nano::state_block & other)
{
	if (handle != nullptr)
	{
		rsnano::rsn_block_destroy (handle);
	}
	cached_hash = other.cached_hash;
	if (other.handle == nullptr)
	{
		handle = nullptr;
	}
	else
	{
		handle = rsnano::rsn_block_clone (other.handle);
	}
	return *this;
}

bool nano::state_block::valid_predecessor (nano::block const & block_a) const
{
	return true;
}

nano::root nano::state_block::root () const
{
	if (!previous ().is_zero ())
	{
		return previous ();
	}
	else
	{
		return account ();
	}
}

nano::link nano::state_block::link () const
{
	uint8_t buffer[32];
	rsnano::rsn_state_block_link (handle, &buffer);
	nano::link result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

nano::account nano::state_block::representative () const
{
	uint8_t buffer[32];
	rsnano::rsn_state_block_representative (handle, &buffer);
	nano::account result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

nano::amount nano::state_block::balance () const
{
	uint8_t buffer[16];
	rsnano::rsn_state_block_balance (handle, &buffer);
	nano::amount result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

void nano::state_block::previous_set (nano::block_hash previous_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (buffer));
	rsnano::rsn_state_block_previous_set (handle, &buffer);
}

void nano::state_block::balance_set (nano::amount balance_a)
{
	uint8_t buffer[16];
	std::copy (std::begin (balance_a.bytes), std::end (balance_a.bytes), std::begin (buffer));
	rsnano::rsn_state_block_balance_set (handle, &buffer);
}

void nano::state_block::account_set (nano::account account_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (buffer));
	rsnano::rsn_state_block_account_set (handle, &buffer);
}

void nano::state_block::representative_set (nano::account account_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (buffer));
	rsnano::rsn_state_block_representative_set (handle, &buffer);
}

void nano::state_block::link_set (nano::link link)
{
	uint8_t buffer[32];
	std::copy (std::begin (link.bytes), std::end (link.bytes), std::begin (buffer));
	rsnano::rsn_state_block_link_set (handle, &buffer);
}

void nano::state_block::zero ()
{
	sign_zero ();
	block_work_set (0);
	account_set (0);
	previous_set (0);
	representative_set (0);
	balance_set (0);
	link_set (0);
}

std::size_t nano::state_block::size ()
{
	return rsnano::rsn_state_block_size ();
}

std::shared_ptr<nano::block> nano::deserialize_block_json (boost::property_tree::ptree const & tree_a, nano::block_uniquer * uniquer_a)
{
	std::shared_ptr<nano::block> result;
	rsnano::BlockHandle * block_handle (rsnano::rsn_deserialize_block_json (&tree_a));
	if (block_handle == nullptr)
		return result;

	result = nano::block_handle_to_block (block_handle);

	if (uniquer_a != nullptr)
	{
		result = uniquer_a->unique (result);
	}
	return result;
}

void nano::serialize_block_type (nano::stream & stream, const nano::block_type & type)
{
	nano::write (stream, type);
}

void nano::serialize_block (nano::stream & stream_a, nano::block const & block_a)
{
	nano::serialize_block_type (stream_a, block_a.type ());
	block_a.serialize (stream_a);
}

std::shared_ptr<nano::block> nano::deserialize_block (nano::stream & stream_a)
{
	nano::block_type type;
	auto error (try_read (stream_a, type));
	std::shared_ptr<nano::block> result;
	if (!error)
	{
		result = nano::deserialize_block (stream_a, type);
	}
	return result;
}

std::shared_ptr<nano::block> nano::deserialize_block (nano::stream & stream_a, nano::block_type type_a, nano::block_uniquer * uniquer_a)
{
	rsnano::BlockUniquerHandle * uniquer_handle = nullptr;
	if (uniquer_a != nullptr)
	{
		uniquer_handle = uniquer_a->handle;
	}
	auto block_handle = rsnano::rsn_deserialize_block (static_cast<uint8_t> (type_a), &stream_a, uniquer_handle);
	if (block_handle == nullptr)
	{
		return nullptr;
	}

	return nano::block_handle_to_block (block_handle);
}

void nano::receive_block::visit (nano::block_visitor & visitor_a) const
{
	visitor_a.receive_block (*this);
}

void nano::receive_block::visit (nano::mutable_block_visitor & visitor_a)
{
	visitor_a.receive_block (*this);
}

bool nano::receive_block::operator== (nano::receive_block const & other_a) const
{
	return rsnano::rsn_block_equals (handle, other_a.handle);
}

nano::receive_block::receive_block ()
{
	rsnano::ReceiveBlockDto dto;
	dto.work = 0;
	handle = rsnano::rsn_receive_block_create (&dto);
}

nano::receive_block::receive_block (nano::block_hash const & previous_a, nano::block_hash const & source_a, nano::raw_key const & prv_a, nano::public_key const & pub_a, uint64_t work_a)
{
	debug_assert (pub_a != nullptr);
	rsnano::ReceiveBlockDto2 dto;
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (dto.previous));
	std::copy (std::begin (source_a.bytes), std::end (source_a.bytes), std::begin (dto.source));
	std::copy (std::begin (prv_a.bytes), std::end (prv_a.bytes), std::begin (dto.priv_key));
	std::copy (std::begin (pub_a.bytes), std::end (pub_a.bytes), std::begin (dto.pub_key));
	dto.work = work_a;
	handle = rsnano::rsn_receive_block_create2 (&dto);
	if (handle == nullptr)
		throw std::runtime_error ("could not create receive_block");
}

nano::receive_block::receive_block (bool & error_a, nano::stream & stream_a)
{
	handle = rsnano::rsn_receive_block_deserialize (&stream_a);
	error_a = handle == nullptr;
}

nano::receive_block::receive_block (bool & error_a, boost::property_tree::ptree const & tree_a)
{
	handle = rsnano::rsn_receive_block_deserialize_json (&tree_a);
	error_a = handle == nullptr;
}

nano::receive_block::receive_block (const nano::receive_block & other)
{
	cached_hash = other.cached_hash;
	if (other.handle == nullptr)
	{
		handle = nullptr;
	}
	else
	{
		handle = rsnano::rsn_block_clone (other.handle);
	}
}

nano::receive_block::receive_block (nano::receive_block && other)
{
	cached_hash = other.cached_hash;
	handle = other.handle;
	other.handle = nullptr;
}

nano::receive_block::receive_block (rsnano::BlockHandle * handle_a)
{
	handle = handle_a;
}

bool nano::receive_block::operator== (nano::block const & other_a) const
{
	return blocks_equal (*this, other_a);
}

bool nano::receive_block::valid_predecessor (nano::block const & block_a) const
{
	bool result;
	switch (block_a.type ())
	{
		case nano::block_type::send:
		case nano::block_type::receive:
		case nano::block_type::open:
		case nano::block_type::change:
			result = true;
			break;
		default:
			result = false;
			break;
	}
	return result;
}

void nano::receive_block::previous_set (nano::block_hash previous_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (previous_a.bytes), std::end (previous_a.bytes), std::begin (buffer));
	rsnano::rsn_receive_block_previous_set (handle, &buffer);
}

nano::block_hash nano::receive_block::source () const
{
	uint8_t buffer[32];
	rsnano::rsn_receive_block_source (handle, &buffer);
	nano::block_hash result;
	std::copy (std::begin (buffer), std::end (buffer), std::begin (result.bytes));
	return result;
}

void nano::receive_block::source_set (nano::block_hash source_a)
{
	uint8_t buffer[32];
	std::copy (std::begin (source_a.bytes), std::end (source_a.bytes), std::begin (buffer));
	rsnano::rsn_receive_block_source_set (handle, &buffer);
}

nano::root nano::receive_block::root () const
{
	return previous ();
}

void nano::receive_block::zero ()
{
	block_work_set (0);
	sign_zero ();
	previous_set (0);
	source_set (0);
}

std::size_t nano::receive_block::size ()
{
	return rsnano::rsn_receive_block_size ();
}

nano::block_details::block_details ()
{
	auto result = rsnano::rsn_block_details_create (static_cast<uint8_t> (nano::epoch::epoch_0), false, false, false, &dto);
	if (result < 0)
	{
		throw std::runtime_error ("could not create block details");
	}
}

nano::block_details::block_details (nano::epoch const epoch_a, bool const is_send_a, bool const is_receive_a, bool const is_epoch_a)
{
	auto result = rsnano::rsn_block_details_create (static_cast<uint8_t> (epoch_a), is_send_a, is_receive_a, is_epoch_a, &dto);
	if (result < 0)
	{
		throw std::runtime_error ("could not create block details");
	}
}

nano::block_details::block_details (rsnano::BlockDetailsDto dto_a)
{
	dto = dto_a;
}

bool nano::block_details::operator== (nano::block_details const & other_a) const
{
	return dto.epoch == other_a.dto.epoch && dto.is_send == other_a.dto.is_send && dto.is_receive == other_a.dto.is_receive && dto.is_epoch == other_a.dto.is_epoch;
}

nano::epoch nano::block_details::epoch () const
{
	return static_cast<nano::epoch> (dto.epoch);
}

bool nano::block_details::is_send () const
{
	return dto.is_send;
}

bool nano::block_details::is_receive () const
{
	return dto.is_receive;
}

bool nano::block_details::is_epoch () const
{
	return dto.is_epoch;
}

void nano::block_details::serialize (nano::stream & stream_a) const
{
	auto result = rsnano::rsn_block_details_serialize (&dto, &stream_a);
	if (result < 0)
	{
		throw new std::runtime_error ("could not serialize block details");
	}
}

bool nano::block_details::deserialize (nano::stream & stream_a)
{
	auto result = rsnano::rsn_block_details_deserialize (&dto, &stream_a);
	return result != 0;
}

std::string nano::state_subtype (nano::block_details const details_a)
{
	debug_assert (details_a.is_epoch () + details_a.is_receive () + details_a.is_send () <= 1);
	if (details_a.is_send ())
	{
		return "send";
	}
	else if (details_a.is_receive ())
	{
		return "receive";
	}
	else if (details_a.is_epoch ())
	{
		return "epoch";
	}
	else
	{
		return "change";
	}
}

nano::block_sideband::block_sideband ()
{
	dto.source_epoch = static_cast<uint8_t> (epoch::epoch_0);
	dto.height = 0;
	dto.timestamp = 0;
	rsnano::rsn_block_details_create (static_cast<uint8_t> (epoch::epoch_0), false, false, false, &dto.details);
	std::fill (std::begin (dto.successor), std::end (dto.successor), 0);
	std::fill (std::begin (dto.account), std::end (dto.account), 0);
	std::fill (std::begin (dto.balance), std::end (dto.balance), 0);
}

nano::block_sideband::block_sideband (rsnano::BlockSidebandDto const & dto_a) :
	dto (dto_a)
{
}

nano::block_sideband::block_sideband (nano::account const & account_a, nano::block_hash const & successor_a, nano::amount const & balance_a, uint64_t const height_a, uint64_t const timestamp_a, nano::block_details const & details_a, nano::epoch const source_epoch_a)
{
	dto.source_epoch = static_cast<uint8_t> (source_epoch_a);
	dto.height = height_a;
	dto.timestamp = timestamp_a;
	dto.details = details_a.dto;
	std::copy (std::begin (successor_a.bytes), std::end (successor_a.bytes), std::begin (dto.successor));
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (dto.account));
	std::copy (std::begin (balance_a.bytes), std::end (balance_a.bytes), std::begin (dto.balance));
}

nano::block_sideband::block_sideband (nano::account const & account_a, nano::block_hash const & successor_a, nano::amount const & balance_a, uint64_t const height_a, uint64_t const timestamp_a, nano::epoch const epoch_a, bool const is_send, bool const is_receive, bool const is_epoch, nano::epoch const source_epoch_a)
{
	dto.source_epoch = static_cast<uint8_t> (source_epoch_a);
	dto.height = height_a;
	dto.timestamp = timestamp_a;
	rsnano::rsn_block_details_create (static_cast<uint8_t> (epoch_a), is_send, is_receive, is_epoch, &dto.details);
	std::copy (std::begin (successor_a.bytes), std::end (successor_a.bytes), std::begin (dto.successor));
	std::copy (std::begin (account_a.bytes), std::end (account_a.bytes), std::begin (dto.account));
	std::copy (std::begin (balance_a.bytes), std::end (balance_a.bytes), std::begin (dto.balance));
}

rsnano::BlockSidebandDto const & nano::block_sideband::as_dto () const
{
	return dto;
}

size_t nano::block_sideband::size (nano::block_type type_a)
{
	int32_t res{ 0 };
	size_t result{ rsnano::rsn_block_sideband_size (static_cast<uint8_t> (type_a), &res) };
	if (res != 0)
		throw std::runtime_error ("rsn_block_sideband_size failed");

	return result;
}

void nano::block_sideband::serialize (nano::stream & stream_a, nano::block_type type_a) const
{
	auto result{ rsnano::rsn_block_sideband_serialize (&dto, &stream_a, static_cast<uint8_t> (type_a)) };
	if (result != 0)
	{
		throw std::runtime_error ("block_sideband serialization failed");
	}
}

bool nano::block_sideband::deserialize (nano::stream & stream_a, nano::block_type type_a)
{
	return rsnano::rsn_block_sideband_deserialize (&dto, &stream_a, static_cast<uint8_t> (type_a)) != 0;
}

nano::epoch nano::block_sideband::source_epoch () const
{
	return static_cast<nano::epoch> (dto.source_epoch);
}

void nano::block_sideband::set_source_epoch (nano::epoch epoch)
{
	dto.source_epoch = static_cast<uint8_t> (epoch);
}

uint64_t nano::block_sideband::height () const
{
	return dto.height;
}

void nano::block_sideband::set_height (uint64_t h)
{
	dto.height = h;
}

uint64_t nano::block_sideband::timestamp () const
{
	return dto.timestamp;
}

void nano::block_sideband::set_timestamp (uint64_t ts)
{
	dto.timestamp = ts;
}

nano::block_details nano::block_sideband::details () const
{
	return nano::block_details (dto.details);
}

nano::block_hash nano::block_sideband::successor () const
{
	nano::block_hash result;
	std::copy (std::begin (dto.successor), std::end (dto.successor), std::begin (result.bytes));
	return result;
}

void nano::block_sideband::set_successor (nano::block_hash successor_a)
{
	std::copy (std::begin (successor_a.bytes), std::end (successor_a.bytes), std::begin (dto.successor));
}

nano::account nano::block_sideband::account () const
{
	nano::account result;
	std::copy (std::begin (dto.account), std::end (dto.account), std::begin (result.bytes));
	return result;
}

nano::amount nano::block_sideband::balance () const
{
	nano::amount result;
	std::copy (std::begin (dto.balance), std::end (dto.balance), std::begin (result.bytes));
	return result;
}

nano::block_uniquer::block_uniquer () :
	handle (rsnano::rsn_block_uniquer_create ())
{
}

nano::block_uniquer::~block_uniquer ()
{
	rsnano::rsn_block_uniquer_destroy (handle);
}

std::shared_ptr<nano::block> nano::block_uniquer::unique (std::shared_ptr<nano::block> const & block_a)
{
	if (block_a == nullptr)
	{
		return nullptr;
	}
	auto uniqued (rsnano::rsn_block_uniquer_unique (handle, block_a->get_handle ()));
	if (uniqued == block_a->get_handle ())
	{
		return block_a;
	}
	else
	{
		return block_handle_to_block (uniqued);
	}
}

size_t nano::block_uniquer::size ()
{
	return rsnano::rsn_block_uniquer_size (handle);
}

std::unique_ptr<nano::container_info_component> nano::collect_container_info (block_uniquer & block_uniquer, std::string const & name)
{
	auto count = block_uniquer.size ();
	auto sizeof_element = sizeof (block_uniquer::value_type);
	auto composite = std::make_unique<container_info_composite> (name);
	composite->add_component (std::make_unique<container_info_leaf> (container_info{ "blocks", count, sizeof_element }));
	return composite;
}

std::shared_ptr<nano::block> nano::block_handle_to_block (rsnano::BlockHandle * handle)
{
	if (handle == nullptr)
		return nullptr;

	auto type = static_cast<nano::block_type> (rsnano::rsn_block_type (handle));
	std::shared_ptr<nano::block> result;
	switch (type)
	{
		case nano::block_type::change:
			result = std::make_shared<nano::change_block> (handle);
			break;

		case nano::block_type::open:
			result = std::make_shared<nano::open_block> (handle);
			break;

		case nano::block_type::receive:
			result = std::make_shared<nano::receive_block> (handle);
			break;

		case nano::block_type::send:
			result = std::make_shared<nano::send_block> (handle);
			break;

		case nano::block_type::state:
			result = std::make_shared<nano::state_block> (handle);
			break;

		default:
			rsnano::rsn_block_destroy (handle);
			throw std::runtime_error ("invalid block type");
	}

	return result;
}
