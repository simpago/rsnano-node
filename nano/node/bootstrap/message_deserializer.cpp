#include <nano/lib/rsnanoutils.hpp>
#include <nano/node/bootstrap/message_deserializer.hpp>
#include <nano/node/node.hpp>

nano::bootstrap::message_deserializer::message_deserializer (nano::network_constants const & network_constants_a, nano::network_filter & publish_filter_a, nano::block_uniquer & block_uniquer_a, nano::vote_uniquer & vote_uniquer_a)
{
	auto constants_dto{ network_constants_a.to_dto () };
	handle_m = rsnano::rsn_message_deserializer_create (&constants_dto, publish_filter_a.handle, block_uniquer_a.handle, vote_uniquer_a.handle);
}

nano::bootstrap::message_deserializer::~message_deserializer ()
{
	rsnano::rsn_message_deserializer_destroy (handle_m);
}

nano::bootstrap::message_deserializer::parse_status nano::bootstrap::message_deserializer::get_status () const
{
	return static_cast<nano::bootstrap::message_deserializer::parse_status> (rsnano::rsn_message_deserializer_status (handle_m));
}

void read_callback_wrapper (void * context_a, const rsnano::ErrorCodeDto * ec_dto_a, rsnano::MessageHandle * msg_handle_a)
{
	try
	{
		auto callback = static_cast<nano::bootstrap::message_deserializer::callback_type *> (context_a);
		auto ec{ rsnano::dto_to_error_code (*ec_dto_a) };
		auto msg{ rsnano::message_handle_to_message (msg_handle_a) };
		(*callback) (ec, std::move (msg));
	}
	catch (std::exception const & e)
	{
		std::cerr << "exception in read_callback_wrapper: " << e.what () << std::endl;
	}
}

void destroy_read_callback (void * context_a)
{
	auto callback = static_cast<nano::bootstrap::message_deserializer::callback_type *> (context_a);
	delete callback;
}

void nano::bootstrap::message_deserializer::read (std::shared_ptr<nano::socket> socket, const nano::bootstrap::message_deserializer::callback_type && callback)
{
	auto context = new nano::bootstrap::message_deserializer::callback_type (callback);
	rsnano::rsn_message_deserializer_read (handle_m, socket->handle, read_callback_wrapper, destroy_read_callback, context);
}

nano::stat::detail nano::bootstrap::message_deserializer::parse_status_to_stat_detail ()
{
	auto detail = rsnano::rsn_message_deserializer_parse_status_to_stat_detail (handle_m);
	return static_cast<nano::stat::detail> (detail);
}

std::string nano::bootstrap::message_deserializer::parse_status_to_string ()
{
	rsnano::StringDto result;
	rsnano::rsn_message_deserializer_parse_status_to_string (handle_m, &result);
	return rsnano::convert_dto_to_string (result);
}
