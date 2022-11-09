#pragma once

#include <nano/boost/asio/ip/tcp.hpp>
#include <nano/boost/asio/ip/udp.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/node/messages.hpp>

namespace nano
{
class message;
}

namespace rsnano
{
boost::system::error_code dto_to_error_code (rsnano::ErrorCodeDto const & dto);
rsnano::ErrorCodeDto error_code_to_dto (boost::system::error_code const & ec);
rsnano::EndpointDto udp_endpoint_to_dto (boost::asio::ip::udp::endpoint const & ep);
rsnano::EndpointDto endpoint_to_dto (boost::asio::ip::tcp::endpoint const & ep);
boost::asio::ip::tcp::endpoint dto_to_endpoint (rsnano::EndpointDto const & dto);
boost::asio::ip::udp::endpoint dto_to_udp_endpoint (rsnano::EndpointDto const & dto);
std::string convert_dto_to_string (rsnano::StringDto & dto);
std::unique_ptr<nano::message> message_handle_to_message (rsnano::MessageHandle * handle);

class io_ctx_wrapper
{
public:
	io_ctx_wrapper (boost::asio::io_context & ctx);
	io_ctx_wrapper (rsnano::IoContextHandle * handle_a);
	io_ctx_wrapper (io_ctx_wrapper const &) = delete;
	~io_ctx_wrapper ();
	rsnano::IoContextHandle * handle () const;
	boost::asio::io_context * inner () const;

private:
	rsnano::IoContextHandle * handle_m;
};

void read_block_array_dto (rsnano::BlockArrayDto & dto, std::vector<std::shared_ptr<nano::block>> & list_a);
rsnano::BlockArrayDto to_block_array_dto (std::vector<std::shared_ptr<nano::block>> & list_a);
}
