#include <nano/node/common.hpp>
#include <nano/secure/buffer.hpp>
#include <nano/test_common/testutil.hpp>

#include <gtest/gtest.h>

#include <boost/property_tree/json_parser.hpp>

#include <crypto/ed25519-donna/ed25519.h>

TEST (sign_message, sign_in_cpp_and_validate_in_rust)
{
	nano::keypair key;
	nano::signature signature;
	nano::uint256_union msg{ 0 };
	ed25519_sign (msg.bytes.data (), msg.bytes.size (), key.prv.bytes.data (), key.pub.bytes.data (), signature.bytes.data ());

	uint8_t priv_key[32];
	uint8_t pub_key[32];
	uint8_t message[32]{ 0 };
	uint8_t rsnano_sig[64]{ 0 };
	uint8_t sig_bytes[64];
	std::copy (std::begin (key.prv.bytes), std::end (key.prv.bytes), std::begin (priv_key));
	std::copy (std::begin (key.pub.bytes), std::end (key.pub.bytes), std::begin (pub_key));
	std::copy (std::begin (signature.bytes), std::end (signature.bytes), std::begin (sig_bytes));

	auto validate_result = rsnano::rsn_validate_message (&pub_key, message, 32, &sig_bytes);
	ASSERT_EQ (validate_result, false);

	message[31] = 1;
	validate_result = rsnano::rsn_validate_message (&pub_key, message, 32, &sig_bytes);
	ASSERT_EQ (validate_result, true);
}

TEST (sign_message, sign_multiple_times)
{
	uint8_t data[] = { 1, 2, 3, 4 };
	nano::keypair key;
	auto signature_a{ nano::sign_message (key.prv, key.pub, &data[0], 4) };
	auto signature_b{ nano::sign_message (key.prv, key.pub, &data[0], 4) };
	ASSERT_NE (signature_a, signature_b);
	bool res_a = nano::validate_message (key.pub, &data[0], 4, signature_a);
	bool res_b = nano::validate_message (key.pub, &data[0], 4, signature_b);
	ASSERT_EQ (res_a, false);
	ASSERT_EQ (res_b, false);
}

TEST (sign_message, sign_in_rust_and_validate_in_cpp)
{
	nano::keypair key;

	uint8_t priv_key[32];
	uint8_t pub_key[32];
	uint8_t message[32]{ 0 };
	uint8_t rsnano_sig[64]{ 0 };
	std::copy (std::begin (key.prv.bytes), std::end (key.prv.bytes), std::begin (priv_key));
	std::copy (std::begin (key.pub.bytes), std::end (key.pub.bytes), std::begin (pub_key));

	auto result{ rsnano::rsn_sign_message (&priv_key, &pub_key, message, 32, &rsnano_sig) };
	ASSERT_EQ (result, 0);

	nano::signature actual;
	std::copy (std::begin (rsnano_sig), std::end (rsnano_sig), std::begin (actual.bytes));
	bool valid = ed25519_sign_open (&message[0], 32, key.pub.bytes.data (), actual.bytes.data ()) == 0;
	ASSERT_EQ (valid, true);
}

TEST (uint512_union, parse_zero)
{
	nano::uint512_union input (nano::uint512_t (0));
	std::string text;
	input.encode_hex (text);
	nano::uint512_union output;
	auto error (output.decode_hex (text));
	ASSERT_FALSE (error);
	ASSERT_EQ (input, output);
	ASSERT_TRUE (output.number ().is_zero ());
}

TEST (uint512_union, parse_zero_short)
{
	std::string text ("0");
	nano::uint512_union output;
	auto error (output.decode_hex (text));
	ASSERT_FALSE (error);
	ASSERT_TRUE (output.number ().is_zero ());
}

TEST (uint512_union, parse_one)
{
	nano::uint512_union input (nano::uint512_t (1));
	std::string text;
	input.encode_hex (text);
	nano::uint512_union output;
	auto error (output.decode_hex (text));
	ASSERT_FALSE (error);
	ASSERT_EQ (input, output);
	ASSERT_EQ (1, output.number ());
}

TEST (uint512_union, parse_error_symbol)
{
	nano::uint512_union input (nano::uint512_t (1000));
	std::string text;
	input.encode_hex (text);
	text[5] = '!';
	nano::uint512_union output;
	auto error (output.decode_hex (text));
	ASSERT_TRUE (error);
}

TEST (uint512_union, max)
{
	nano::uint512_union input (std::numeric_limits<nano::uint512_t>::max ());
	std::string text;
	input.encode_hex (text);
	nano::uint512_union output;
	auto error (output.decode_hex (text));
	ASSERT_FALSE (error);
	ASSERT_EQ (input, output);
	ASSERT_EQ (nano::uint512_t ("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"), output.number ());
}

TEST (uint512_union, parse_error_overflow)
{
	nano::uint512_union input (std::numeric_limits<nano::uint512_t>::max ());
	std::string text;
	input.encode_hex (text);
	text.push_back (0);
	nano::uint512_union output;
	auto error (output.decode_hex (text));
	ASSERT_TRUE (error);
}

TEST (block, difficulty)
{
	nano::keypair key;
	nano::send_block block (0, 1, 2, key.prv, key.pub, 5);
	ASSERT_EQ (nano::dev::network_params.work.difficulty (block), nano::dev::network_params.work.difficulty (block.work_version (), block.root (), block.block_work ()));
}

TEST (blocks, work_version)
{
	ASSERT_EQ (nano::work_version::work_1, nano::send_block ().work_version ());
	ASSERT_EQ (nano::work_version::work_1, nano::receive_block ().work_version ());
	ASSERT_EQ (nano::work_version::work_1, nano::change_block ().work_version ());
	ASSERT_EQ (nano::work_version::work_1, nano::open_block ().work_version ());
	ASSERT_EQ (nano::work_version::work_1, nano::state_block ().work_version ());
}

TEST (block_uniquer, null)
{
	nano::block_uniquer uniquer;
	ASSERT_EQ (nullptr, uniquer.unique (nullptr));
}

TEST (block_builder, state_missing_rep)
{
	// Test against a random hash from the live network
	std::error_code ec;
	nano::block_builder builder;
	auto block = builder
				 .state ()
				 .account_address ("xrb_15nhh1kzw3x8ohez6s75wy3jr6dqgq65oaede1fzk5hqxk4j8ehz7iqtb3to")
				 .previous_hex ("FEFBCE274E75148AB31FF63EFB3082EF1126BF72BF3FA9C76A97FD5A9F0EBEC5")
				 .balance_dec ("2251569974100400000000000000000000")
				 .link_hex ("E16DD58C1EFA8B521545B0A74375AA994D9FC43828A4266D75ECF57F07A7EE86")
				 .sign_zero ()
				 .work (0)
				 .build (ec);
	ASSERT_EQ (ec, nano::error_common::missing_representative);
}

TEST (block_builder, state_errors)
{
	std::error_code ec;
	nano::block_builder builder;

	// Ensure the proper error is generated
	builder.state ().account_hex ("xrb_bad").build (ec);
	ASSERT_EQ (ec, nano::error_common::bad_account_number);

	builder.state ().zero ().account_address ("xrb_1111111111111111111111111111111111111111111111111111hifc8npp").build (ec);
	ASSERT_NO_ERROR (ec);
}

TEST (block_builder, open)
{
	// Test built block's hash against the Genesis open block from the live network
	std::error_code ec;
	nano::block_builder builder;
	auto block = builder
				 .open ()
				 .account_address ("xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3")
				 .representative_address ("xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3")
				 .source_hex ("E89208DD038FBB269987689621D52292AE9C35941A7484756ECCED92A65093BA")
				 .build (ec);
	ASSERT_EQ (block->hash ().to_string (), "991CF190094C00F0B68E2E5F75F6BEE95A2E0BD93CEAA4A6734DB9F19B728948");
	ASSERT_EQ (block->source ().to_string (), "E89208DD038FBB269987689621D52292AE9C35941A7484756ECCED92A65093BA");
	ASSERT_TRUE (block->destination ().is_zero ());
	ASSERT_TRUE (block->link ().is_zero ());
}

TEST (block_builder, open_equality)
{
	std::error_code ec;
	nano::block_builder builder;

	// With constructor
	nano::keypair key1, key2;
	nano::open_block block1 (1, key1.pub, key2.pub, key1.prv, key1.pub, 5);

	// With builder
	auto block2 = builder
				  .open ()
				  .source (1)
				  .account (key2.pub)
				  .representative (key1.pub)
				  .sign (key1.prv, key1.pub)
				  .work (5)
				  .build (ec);

	ASSERT_NO_ERROR (ec);
	ASSERT_EQ (block1.hash (), block2->hash ());
	ASSERT_EQ (block1.block_work (), block2->block_work ());
}

TEST (block_builder, change)
{
	std::error_code ec;
	nano::block_builder builder;
	auto block = builder
				 .change ()
				 .representative_address ("xrb_3rropjiqfxpmrrkooej4qtmm1pueu36f9ghinpho4esfdor8785a455d16nf")
				 .previous_hex ("088EE46429CA936F76C4EAA20B97F6D33E5D872971433EE0C1311BCB98764456")
				 .build (ec);
	ASSERT_EQ (block->hash ().to_string (), "13552AC3928E93B5C6C215F61879358E248D4A5246B8B3D1EEC5A566EDCEE077");
	ASSERT_TRUE (block->source ().is_zero ());
	ASSERT_TRUE (block->destination ().is_zero ());
	ASSERT_TRUE (block->link ().is_zero ());
}

TEST (block_builder, change_equality)
{
	std::error_code ec;
	nano::block_builder builder;

	// With constructor
	nano::keypair key1, key2;
	nano::change_block block1 (1, key1.pub, key1.prv, key1.pub, 5);

	// With builder
	auto block2 = builder
				  .change ()
				  .previous (1)
				  .representative (key1.pub)
				  .sign (key1.prv, key1.pub)
				  .work (5)
				  .build (ec);

	ASSERT_NO_ERROR (ec);
	ASSERT_EQ (block1.hash (), block2->hash ());
	ASSERT_EQ (block1.block_work (), block2->block_work ());
}

TEST (block_builder, send)
{
	std::error_code ec;
	nano::block_builder builder;
	auto block = builder
				 .send ()
				 .destination_address ("xrb_1gys8r4crpxhp94n4uho5cshaho81na6454qni5gu9n53gksoyy1wcd4udyb")
				 .previous_hex ("F685856D73A488894F7F3A62BC3A88E17E985F9969629FF3FDD4A0D4FD823F24")
				 .balance_hex ("00F035A9C7D818E7C34148C524FFFFEE")
				 .build (ec);
	ASSERT_EQ (block->hash ().to_string (), "4560E7B1F3735D082700CFC2852F5D1F378F7418FD24CEF1AD45AB69316F15CD");
	ASSERT_TRUE (block->source ().is_zero ());
	ASSERT_EQ (block->destination ().to_account (), "nano_1gys8r4crpxhp94n4uho5cshaho81na6454qni5gu9n53gksoyy1wcd4udyb");
	ASSERT_TRUE (block->link ().is_zero ());
}

TEST (block_builder, send_equality)
{
	std::error_code ec;
	nano::block_builder builder;

	// With constructor
	nano::keypair key1, key2;
	nano::send_block block1 (1, key1.pub, 2, key1.prv, key1.pub, 5);

	// With builder
	auto block2 = builder
				  .send ()
				  .previous (1)
				  .destination (key1.pub)
				  .balance (2)
				  .sign (key1.prv, key1.pub)
				  .work (5)
				  .build (ec);

	ASSERT_NO_ERROR (ec);
	ASSERT_EQ (block1.hash (), block2->hash ());
	ASSERT_EQ (block1.block_work (), block2->block_work ());
}

TEST (block_builder, receive_equality)
{
	std::error_code ec;
	nano::block_builder builder;

	// With constructor
	nano::keypair key1;
	nano::receive_block block1 (1, 2, key1.prv, key1.pub, 5);

	// With builder
	auto block2 = builder
				  .receive ()
				  .previous (1)
				  .source (2)
				  .sign (key1.prv, key1.pub)
				  .work (5)
				  .build (ec);

	ASSERT_NO_ERROR (ec);
	ASSERT_EQ (block1.hash (), block2->hash ());
	ASSERT_EQ (block1.block_work (), block2->block_work ());
}

TEST (block_builder, receive)
{
	std::error_code ec;
	nano::block_builder builder;
	auto block = builder
				 .receive ()
				 .previous_hex ("59660153194CAC5DAC08509D87970BF86F6AEA943025E2A7ED7460930594950E")
				 .source_hex ("7B2B0A29C1B235FDF9B4DEF2984BB3573BD1A52D28246396FBB3E4C5FE662135")
				 .build (ec);
	ASSERT_EQ (block->hash ().to_string (), "6C004BF911D9CF2ED75CF6EC45E795122AD5D093FF5A83EDFBA43EC4A3EDC722");
	ASSERT_EQ (block->source ().to_string (), "7B2B0A29C1B235FDF9B4DEF2984BB3573BD1A52D28246396FBB3E4C5FE662135");
	ASSERT_TRUE (block->destination ().is_zero ());
	ASSERT_TRUE (block->link ().is_zero ());
}
