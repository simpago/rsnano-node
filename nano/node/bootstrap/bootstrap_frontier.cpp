#include "nano/lib/numbers.hpp"
#include "nano/lib/rsnano.hpp"

#include <nano/node/bootstrap/bootstrap_frontier.hpp>
#include <nano/node/node.hpp>
#include <nano/node/transport/tcp.hpp>
#include <nano/secure/ledger.hpp>

#include <boost/format.hpp>

constexpr double nano::bootstrap_limits::bootstrap_connection_warmup_time_sec;
constexpr double nano::bootstrap_limits::bootstrap_minimum_elapsed_seconds_blockrate;
constexpr double nano::bootstrap_limits::bootstrap_minimum_frontier_blocks_per_sec;
constexpr unsigned nano::bootstrap_limits::bulk_push_cost_limit;

//------------------------------------------------------------------------------
// frontier_req_server
//------------------------------------------------------------------------------

namespace
{
rsnano::FrontierReqServerHandle * create_frontier_req_server_handle (
std::shared_ptr<nano::node> const & node_a,
std::shared_ptr<nano::transport::tcp_server> const & connection_a,
rsnano::MessageHandle * request_a)
{
	return rsnano::rsn_frontier_req_server_create (connection_a->handle,
	request_a,
	node_a->bootstrap_workers->handle,
	node_a->ledger.get_handle (),
	node_a->async_rt.handle);
}
}

nano::frontier_req_server::frontier_req_server (std::shared_ptr<nano::node> const & node_a, std::shared_ptr<nano::transport::tcp_server> const & connection_a, std::unique_ptr<nano::frontier_req> request_a) :
	handle{ create_frontier_req_server_handle (node_a, connection_a, request_a->handle) }
{
}

nano::frontier_req_server::~frontier_req_server ()
{
	rsnano::rsn_frontier_req_server_destroy (handle);
}

nano::public_key nano::frontier_req_server::current () const
{
	nano::public_key result;
	rsnano::rsn_frontier_req_server_current (handle, result.bytes.data ());
	return result;
}

nano::block_hash nano::frontier_req_server::frontier () const
{
	nano::block_hash result;
	rsnano::rsn_frontier_req_server_frontier (handle, result.bytes.data ());
	return result;
}
