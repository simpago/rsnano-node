#pragma once

#include "nano/lib/numbers.hpp"

#include <nano/node/common.hpp>

#include <memory>

namespace rsnano
{
class FrontierReqClientHandle;
}

namespace nano
{
class bootstrap_client;
class node;
namespace transport
{
	class tcp_server;
}

class frontier_req;

/**
 * Server side of a frontier request. Created when a tcp_server receives a frontier_req message and exited when end-of-list is reached.
 */
class frontier_req_server final : public std::enable_shared_from_this<nano::frontier_req_server>
{
public:
	frontier_req_server (std::shared_ptr<nano::node> const &, std::shared_ptr<nano::transport::tcp_server> const &, std::unique_ptr<nano::frontier_req>);
	frontier_req_server (frontier_req_server const &) = delete;
	~frontier_req_server ();
	nano::public_key current () const;
	nano::block_hash frontier () const;
	rsnano::FrontierReqServerHandle * handle;
};
}
