#include <nano/crypto_lib/random_pool.hpp>
#include <nano/crypto_lib/secure_memory.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/utility.hpp>
#include <nano/secure/common.hpp>

void nano::public_key::encode_account (std::string & destination_a) const
{
	uint8_t account_bytes[32];
	uint8_t string_bytes[65];

	std::copy (std::begin (bytes), std::end (bytes), std::begin (account_bytes));
	rsnano::rsn_account_encode (&account_bytes, &string_bytes);
	destination_a.reserve (65);
	destination_a.append (reinterpret_cast<const char *> (string_bytes), 65);
}

std::string nano::public_key::to_account () const
{
	std::string result;
	encode_account (result);
	return result;
}

nano::public_key::public_key () :
	uint256_union{ 0 }
{
}

nano::public_key const & nano::public_key::null ()
{
	return nano::hardened_constants::get ().not_an_account;
}

std::string nano::public_key::to_node_id () const
{
	return to_account ().replace (0, 4, "node");
}

bool nano::public_key::decode_node_id (std::string const & source_a)
{
	return decode_account (source_a);
}

bool nano::public_key::decode_account (std::string const & source_a)
{
	uint8_t result[32];
	if (rsnano::rsn_account_decode (source_a.c_str (), &result) < 0)
	{
		return true;
	}
	std::copy (std::begin (result), std::end (result), std::begin (bytes));

	return false;
}

nano::uint256_union::uint256_union (nano::uint256_t const & number_a)
{
	bytes.fill (0);
	boost::multiprecision::export_bits (number_a, bytes.rbegin (), 8, false);
}

// Construct a uint256_union = AES_ENC_CTR (cleartext, key, iv)
void nano::uint256_union::encrypt (nano::raw_key const & cleartext, nano::raw_key const & key, uint128_union const & iv)
{
	rsnano::rsn_raw_key_encrypt (bytes.data (), cleartext.bytes.data (), key.bytes.data (), iv.bytes.data ());
}

bool nano::uint256_union::is_zero () const
{
	return qwords[0] == 0 && qwords[1] == 0 && qwords[2] == 0 && qwords[3] == 0;
}

std::string nano::uint256_union::to_string () const
{
	std::string result;
	encode_hex (result);
	return result;
}

nano::uint256_union & nano::uint256_union::operator^= (nano::uint256_union const & other_a)
{
	auto j (other_a.qwords.begin ());
	for (auto i (qwords.begin ()), n (qwords.end ()); i != n; ++i, ++j)
	{
		*i ^= *j;
	}
	return *this;
}

nano::uint256_union nano::uint256_union::operator^ (nano::uint256_union const & other_a) const
{
	nano::uint256_union result;
	auto k (result.qwords.begin ());
	for (auto i (qwords.begin ()), j (other_a.qwords.begin ()), n (qwords.end ()); i != n; ++i, ++j, ++k)
	{
		*k = *i ^ *j;
	}
	return result;
}

nano::uint256_union::uint256_union (std::string const & hex_a)
{
	auto error (decode_hex (hex_a));

	release_assert (!error);
}

void nano::uint256_union::clear ()
{
	qwords.fill (0);
}

nano::uint256_t nano::uint256_union::number () const
{
	nano::uint256_t result;
	boost::multiprecision::import_bits (result, bytes.begin (), bytes.end ());
	return result;
}

void nano::uint256_union::encode_hex (std::string & text) const
{
	debug_assert (text.empty ());
	std::stringstream stream;
	stream << std::hex << std::uppercase << std::noshowbase << std::setw (64) << std::setfill ('0');
	stream << number ();
	text = stream.str ();
}

bool nano::uint256_union::decode_hex (std::string const & text)
{
	auto error (false);
	if (!text.empty () && text.size () <= 64)
	{
		std::stringstream stream (text);
		stream << std::hex << std::noshowbase;
		nano::uint256_t number_l;
		try
		{
			stream >> number_l;
			*this = number_l;
			if (!stream.eof ())
			{
				error = true;
			}
		}
		catch (std::runtime_error &)
		{
			error = true;
		}
	}
	else
	{
		error = true;
	}
	return error;
}

void nano::uint256_union::encode_dec (std::string & text) const
{
	debug_assert (text.empty ());
	std::stringstream stream;
	stream << std::dec << std::noshowbase;
	stream << number ();
	text = stream.str ();
}

bool nano::uint256_union::decode_dec (std::string const & text)
{
	auto error (text.size () > 78 || (text.size () > 1 && text.front () == '0') || (!text.empty () && text.front () == '-'));
	if (!error)
	{
		std::stringstream stream (text);
		stream << std::dec << std::noshowbase;
		nano::uint256_t number_l;
		try
		{
			stream >> number_l;
			*this = number_l;
			if (!stream.eof ())
			{
				error = true;
			}
		}
		catch (std::runtime_error &)
		{
			error = true;
		}
	}
	return error;
}

nano::uint256_union::uint256_union (uint64_t value0)
{
	*this = nano::uint256_t (value0);
}

bool nano::uint512_union::operator== (nano::uint512_union const & other_a) const
{
	return bytes == other_a.bytes;
}

nano::uint512_union::uint512_union (nano::uint256_union const & upper, nano::uint256_union const & lower)
{
	uint256s[0] = upper;
	uint256s[1] = lower;
}

nano::uint512_union::uint512_union (nano::uint512_t const & number_a)
{
	bytes.fill (0);
	boost::multiprecision::export_bits (number_a, bytes.rbegin (), 8, false);
}

bool nano::uint512_union::is_zero () const
{
	return qwords[0] == 0 && qwords[1] == 0 && qwords[2] == 0 && qwords[3] == 0
	&& qwords[4] == 0 && qwords[5] == 0 && qwords[6] == 0 && qwords[7] == 0;
}

void nano::uint512_union::clear ()
{
	bytes.fill (0);
}

nano::uint512_t nano::uint512_union::number () const
{
	nano::uint512_t result;
	boost::multiprecision::import_bits (result, bytes.begin (), bytes.end ());
	return result;
}

void nano::uint512_union::encode_hex (std::string & text) const
{
	debug_assert (text.empty ());
	std::stringstream stream;
	stream << std::hex << std::uppercase << std::noshowbase << std::setw (128) << std::setfill ('0');
	stream << number ();
	text = stream.str ();
}

bool nano::uint512_union::decode_hex (std::string const & text)
{
	auto error (text.size () > 128);
	if (!error)
	{
		std::stringstream stream (text);
		stream << std::hex << std::noshowbase;
		nano::uint512_t number_l;
		try
		{
			stream >> number_l;
			*this = number_l;
			if (!stream.eof ())
			{
				error = true;
			}
		}
		catch (std::runtime_error &)
		{
			error = true;
		}
	}
	return error;
}

bool nano::uint512_union::operator!= (nano::uint512_union const & other_a) const
{
	return !(*this == other_a);
}

nano::uint512_union & nano::uint512_union::operator^= (nano::uint512_union const & other_a)
{
	uint256s[0] ^= other_a.uint256s[0];
	uint256s[1] ^= other_a.uint256s[1];
	return *this;
}

std::string nano::uint512_union::to_string () const
{
	std::string result;
	encode_hex (result);
	return result;
}

nano::raw_key::~raw_key ()
{
	secure_wipe_memory (bytes.data (), bytes.size ());
}

void nano::raw_key::decrypt (nano::uint256_union const & ciphertext, nano::raw_key const & key_a, uint128_union const & iv)
{
	rsnano::rsn_raw_key_decrypt (bytes.data (), ciphertext.bytes.data (), key_a.bytes.data (), iv.bytes.data ());
}

nano::raw_key nano::deterministic_key (nano::raw_key const & seed_a, uint32_t index_a)
{
	nano::raw_key prv_key;
	rsnano::rsn_deterministic_key (seed_a.bytes.data (), index_a, prv_key.bytes.data ());
	return prv_key;
}

nano::public_key nano::pub_key (nano::raw_key const & raw_key_a)
{
	nano::public_key result;
	rsnano::rsn_pub_key (raw_key_a.bytes.data (), result.bytes.data ());
	return result;
}

nano::signature nano::sign_message (nano::raw_key const & private_key, nano::public_key const & public_key, uint8_t const * data, size_t size)
{
	nano::signature result;
	if (rsnano::rsn_sign_message (private_key.bytes.data (), public_key.bytes.data (), data, size, result.bytes.data ()) != 0)
		throw std::runtime_error ("could not sign message");
	return result;
}

nano::signature nano::sign_message (nano::raw_key const & private_key, nano::public_key const & public_key, nano::uint256_union const & message)
{
	return nano::sign_message (private_key, public_key, message.bytes.data (), sizeof (message.bytes));
}

bool nano::validate_message (nano::public_key const & public_key, uint8_t const * data, size_t size, nano::signature const & signature)
{
	return rsnano::rsn_validate_message ((uint8_t (*)[32])public_key.bytes.data (), data, size, (uint8_t (*)[64])signature.bytes.data ());
}

bool nano::validate_message (nano::public_key const & public_key, nano::uint256_union const & message, nano::signature const & signature)
{
	return validate_message (public_key, message.bytes.data (), sizeof (message.bytes), signature);
}

bool nano::validate_message_batch (const unsigned char ** m, size_t * mlen, const unsigned char ** pk, const unsigned char ** RS, size_t num, int * valid)
{
	return rsnano::rsn_validate_batch (m, mlen, pk, RS, num, valid);
}

nano::uint128_union::uint128_union (std::string const & string_a)
{
	auto error (decode_hex (string_a));

	release_assert (!error);
}

nano::uint128_union::uint128_union (uint64_t value_a)
{
	*this = nano::uint128_t (value_a);
}

nano::uint128_union::uint128_union (nano::uint128_t const & number_a)
{
	bytes.fill (0);
	boost::multiprecision::export_bits (number_a, bytes.rbegin (), 8, false);
}

bool nano::uint128_union::operator== (nano::uint128_union const & other_a) const
{
	return qwords[0] == other_a.qwords[0] && qwords[1] == other_a.qwords[1];
}

bool nano::uint128_union::operator!= (nano::uint128_union const & other_a) const
{
	return !(*this == other_a);
}

bool nano::uint128_union::operator< (nano::uint128_union const & other_a) const
{
	return std::memcmp (bytes.data (), other_a.bytes.data (), 16) < 0;
}

bool nano::uint128_union::operator> (nano::uint128_union const & other_a) const
{
	return std::memcmp (bytes.data (), other_a.bytes.data (), 16) > 0;
}

nano::uint128_t nano::uint128_union::number () const
{
	nano::uint128_t result;
	boost::multiprecision::import_bits (result, bytes.begin (), bytes.end ());
	return result;
}

void nano::uint128_union::encode_hex (std::string & text) const
{
	debug_assert (text.empty ());
	std::stringstream stream;
	stream << std::hex << std::uppercase << std::noshowbase << std::setw (32) << std::setfill ('0');
	stream << number ();
	text = stream.str ();
}

bool nano::uint128_union::decode_hex (std::string const & text)
{
	auto error (text.size () > 32);
	if (!error)
	{
		std::stringstream stream (text);
		stream << std::hex << std::noshowbase;
		nano::uint128_t number_l;
		try
		{
			stream >> number_l;
			*this = number_l;
			if (!stream.eof ())
			{
				error = true;
			}
		}
		catch (std::runtime_error &)
		{
			error = true;
		}
	}
	return error;
}

void nano::uint128_union::encode_dec (std::string & text) const
{
	debug_assert (text.empty ());
	std::stringstream stream;
	stream << std::dec << std::noshowbase;
	stream << number ();
	text = stream.str ();
}

bool nano::uint128_union::decode_dec (std::string const & text, bool decimal)
{
	auto error (text.size () > 39 || (text.size () > 1 && text.front () == '0' && !decimal) || (!text.empty () && text.front () == '-'));
	if (!error)
	{
		std::stringstream stream (text);
		stream << std::dec << std::noshowbase;
		boost::multiprecision::checked_uint128_t number_l;
		try
		{
			stream >> number_l;
			nano::uint128_t unchecked (number_l);
			*this = unchecked;
			if (!stream.eof ())
			{
				error = true;
			}
		}
		catch (std::runtime_error &)
		{
			error = true;
		}
	}
	return error;
}

bool nano::uint128_union::decode_dec (std::string const & text, nano::uint128_t scale)
{
	bool error (text.size () > 40 || (!text.empty () && text.front () == '-'));
	if (!error)
	{
		auto delimiter_position (text.find (".")); // Dot delimiter hardcoded until decision for supporting other locales
		if (delimiter_position == std::string::npos)
		{
			nano::uint128_union integer;
			error = integer.decode_dec (text);
			if (!error)
			{
				// Overflow check
				try
				{
					auto result (boost::multiprecision::checked_uint128_t (integer.number ()) * boost::multiprecision::checked_uint128_t (scale));
					error = (result > std::numeric_limits<nano::uint128_t>::max ());
					if (!error)
					{
						*this = nano::uint128_t (result);
					}
				}
				catch (std::overflow_error &)
				{
					error = true;
				}
			}
		}
		else
		{
			nano::uint128_union integer_part;
			std::string integer_text (text.substr (0, delimiter_position));
			error = (integer_text.empty () || integer_part.decode_dec (integer_text));
			if (!error)
			{
				// Overflow check
				try
				{
					error = ((boost::multiprecision::checked_uint128_t (integer_part.number ()) * boost::multiprecision::checked_uint128_t (scale)) > std::numeric_limits<nano::uint128_t>::max ());
				}
				catch (std::overflow_error &)
				{
					error = true;
				}
				if (!error)
				{
					nano::uint128_union decimal_part;
					std::string decimal_text (text.substr (delimiter_position + 1, text.length ()));
					error = (decimal_text.empty () || decimal_part.decode_dec (decimal_text, true));
					if (!error)
					{
						// Overflow check
						auto scale_length (scale.convert_to<std::string> ().length ());
						error = (scale_length <= decimal_text.length ());
						if (!error)
						{
							auto base10 = boost::multiprecision::cpp_int (10);
							release_assert ((scale_length - decimal_text.length () - 1) <= std::numeric_limits<unsigned>::max ());
							auto pow10 = boost::multiprecision::pow (base10, static_cast<unsigned> (scale_length - decimal_text.length () - 1));
							auto decimal_part_num = decimal_part.number ();
							auto integer_part_scaled = integer_part.number () * scale;
							auto decimal_part_mult_pow = decimal_part_num * pow10;
							auto result = integer_part_scaled + decimal_part_mult_pow;

							// Overflow check
							error = (result > std::numeric_limits<nano::uint128_t>::max ());
							if (!error)
							{
								*this = nano::uint128_t (result);
							}
						}
					}
				}
			}
		}
	}
	return error;
}

void format_frac (std::ostringstream & stream, nano::uint128_t value, nano::uint128_t scale, int precision)
{
	auto reduce = scale;
	auto rem = value;
	while (reduce > 1 && rem > 0 && precision > 0)
	{
		reduce /= 10;
		auto val = rem / reduce;
		rem -= val * reduce;
		stream << val;
		precision--;
	}
}

void format_dec (std::ostringstream & stream, nano::uint128_t value, char group_sep, std::string const & groupings)
{
	auto largestPow10 = nano::uint256_t (1);
	int dec_count = 1;
	while (1)
	{
		auto next = largestPow10 * 10;
		if (next > value)
		{
			break;
		}
		largestPow10 = next;
		dec_count++;
	}

	if (dec_count > 39)
	{
		// Impossible.
		return;
	}

	// This could be cached per-locale.
	bool emit_group[39];
	if (group_sep != 0)
	{
		int group_index = 0;
		int group_count = 0;
		for (int i = 0; i < dec_count; i++)
		{
			group_count++;
			if (group_count > groupings[group_index])
			{
				group_index = std::min (group_index + 1, (int)groupings.length () - 1);
				group_count = 1;
				emit_group[i] = true;
			}
			else
			{
				emit_group[i] = false;
			}
		}
	}

	auto reduce = nano::uint128_t (largestPow10);
	nano::uint128_t rem = value;
	while (reduce > 0)
	{
		auto val = rem / reduce;
		rem -= val * reduce;
		stream << val;
		dec_count--;
		if (group_sep != 0 && emit_group[dec_count] && reduce > 1)
		{
			stream << group_sep;
		}
		reduce /= 10;
	}
}

std::string format_balance (nano::uint128_t balance, nano::uint128_t scale, int precision, bool group_digits, char thousands_sep, char decimal_point, std::string & grouping)
{
	std::ostringstream stream;
	auto int_part = balance / scale;
	auto frac_part = balance % scale;
	auto prec_scale = scale;
	for (int i = 0; i < precision; i++)
	{
		prec_scale /= 10;
	}
	if (int_part == 0 && frac_part > 0 && frac_part / prec_scale == 0)
	{
		// Display e.g. "< 0.01" rather than 0.
		stream << "< ";
		if (precision > 0)
		{
			stream << "0";
			stream << decimal_point;
			for (int i = 0; i < precision - 1; i++)
			{
				stream << "0";
			}
		}
		stream << "1";
	}
	else
	{
		format_dec (stream, int_part, group_digits && grouping.length () > 0 ? thousands_sep : 0, grouping);
		if (precision > 0 && frac_part > 0)
		{
			stream << decimal_point;
			format_frac (stream, frac_part, scale, precision);
		}
	}
	return stream.str ();
}

std::string nano::uint128_union::format_balance (nano::uint128_t scale, int precision, bool group_digits) const
{
	auto thousands_sep = std::use_facet<std::numpunct<char>> (std::locale ()).thousands_sep ();
	auto decimal_point = std::use_facet<std::numpunct<char>> (std::locale ()).decimal_point ();
	std::string grouping = "\3";
	return ::format_balance (number (), scale, precision, group_digits, thousands_sep, decimal_point, grouping);
}

std::string nano::uint128_union::format_balance (nano::uint128_t scale, int precision, bool group_digits, std::locale const & locale) const
{
	auto thousands_sep = std::use_facet<std::moneypunct<char>> (locale).thousands_sep ();
	auto decimal_point = std::use_facet<std::moneypunct<char>> (locale).decimal_point ();
	std::string grouping = std::use_facet<std::moneypunct<char>> (locale).grouping ();
	return ::format_balance (number (), scale, precision, group_digits, thousands_sep, decimal_point, grouping);
}

void nano::uint128_union::clear ()
{
	qwords.fill (0);
}

bool nano::uint128_union::is_zero () const
{
	return qwords[0] == 0 && qwords[1] == 0;
}

std::string nano::uint128_union::to_string () const
{
	std::string result;
	encode_hex (result);
	return result;
}

std::string nano::uint128_union::to_string_dec () const
{
	std::string result;
	encode_dec (result);
	return result;
}

nano::hash_or_account::hash_or_account () :
	account{}
{
}

nano::hash_or_account::hash_or_account (uint64_t value_a) :
	raw (value_a)
{
}

bool nano::hash_or_account::is_zero () const
{
	return raw.is_zero ();
}

void nano::hash_or_account::clear ()
{
	raw.clear ();
}

bool nano::hash_or_account::decode_hex (std::string const & text_a)
{
	return raw.decode_hex (text_a);
}

bool nano::hash_or_account::decode_account (std::string const & source_a)
{
	return account.decode_account (source_a);
}

std::string nano::hash_or_account::to_string () const
{
	return raw.to_string ();
}

std::string nano::hash_or_account::to_account () const
{
	return account.to_account ();
}

nano::block_hash const & nano::hash_or_account::as_block_hash () const
{
	return hash;
}

nano::account const & nano::hash_or_account::as_account () const
{
	return account;
}

nano::hash_or_account::operator nano::uint256_union const & () const
{
	return raw;
}

nano::block_hash const & nano::root::previous () const
{
	return hash;
}

bool nano::hash_or_account::operator== (nano::hash_or_account const & hash_or_account_a) const
{
	return bytes == hash_or_account_a.bytes;
}

bool nano::hash_or_account::operator!= (nano::hash_or_account const & hash_or_account_a) const
{
	return !(*this == hash_or_account_a);
}

std::string nano::to_string_hex (uint64_t const value_a)
{
	std::stringstream stream;
	stream << std::hex << std::noshowbase << std::setw (16) << std::setfill ('0');
	stream << value_a;
	return stream.str ();
}

std::string nano::to_string_hex (uint16_t const value_a)
{
	std::stringstream stream;
	stream << std::hex << std::noshowbase << std::setw (4) << std::setfill ('0');
	stream << value_a;
	return stream.str ();
}

bool nano::from_string_hex (std::string const & value_a, uint64_t & target_a)
{
	auto error (value_a.empty ());
	if (!error)
	{
		error = value_a.size () > 16;
		if (!error)
		{
			std::stringstream stream (value_a);
			stream << std::hex << std::noshowbase;
			try
			{
				uint64_t number_l;
				stream >> number_l;
				target_a = number_l;
				if (!stream.eof ())
				{
					error = true;
				}
			}
			catch (std::runtime_error &)
			{
				error = true;
			}
		}
	}
	return error;
}

std::string nano::to_string (double const value_a, int const precision_a)
{
	std::stringstream stream;
	stream << std::setprecision (precision_a) << std::fixed;
	stream << value_a;
	return stream.str ();
}

uint64_t nano::difficulty::from_multiplier (double const multiplier_a, uint64_t const base_difficulty_a)
{
	return rsnano::rsn_difficulty_from_multiplier (multiplier_a, base_difficulty_a);
}

double nano::difficulty::to_multiplier (uint64_t const difficulty_a, uint64_t const base_difficulty_a)
{
	return rsnano::rsn_difficulty_to_multiplier (difficulty_a, base_difficulty_a);
}

nano::public_key::operator nano::link const & () const
{
	return reinterpret_cast<nano::link const &> (*this);
}

nano::public_key::operator nano::root const & () const
{
	return reinterpret_cast<nano::root const &> (*this);
}

nano::public_key::operator nano::hash_or_account const & () const
{
	return reinterpret_cast<nano::hash_or_account const &> (*this);
}

bool nano::public_key::operator== (std::nullptr_t) const
{
	return bytes == null ().bytes;
}

bool nano::public_key::operator!= (std::nullptr_t) const
{
	return !(*this == nullptr);
}

nano::block_hash::operator nano::link const & () const
{
	return reinterpret_cast<nano::link const &> (*this);
}

nano::block_hash::operator nano::root const & () const
{
	return reinterpret_cast<nano::root const &> (*this);
}

nano::block_hash::operator nano::hash_or_account const & () const
{
	return reinterpret_cast<nano::hash_or_account const &> (*this);
}
