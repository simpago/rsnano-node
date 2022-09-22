#include <nano/node/node.hpp>
#include <nano/node/transport/fake.hpp>

#include <boost/format.hpp>

nano::transport::fake::channel::channel (nano::node & node) :
	node{ node },
	transport::channel{ rsnano::rsn_channel_fake_create (std::chrono::steady_clock::now ().time_since_epoch ().count ()) },
	endpoint{ node.network->endpoint () }
{
	set_node_id (node.node_id.pub);
	set_network_version (node.network_params.network.protocol_version);
}

void nano::transport::fake::channel::send (nano::message & message_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::buffer_drop_policy drop_policy_a)
{
	auto buffer (message_a.to_shared_const_buffer ());
	auto detail = nano::message_type_to_stat_detail (message_a.get_header ().get_type ());
	auto is_droppable_by_limiter = drop_policy_a == nano::buffer_drop_policy::limiter;
	auto should_drop (node.network->limiter.should_drop (buffer.size ()));
	if (!is_droppable_by_limiter || !should_drop)
	{
		send_buffer (buffer, callback_a, drop_policy_a);
		node.stats->inc (nano::stat::type::message, detail, nano::stat::dir::out);
	}
	else
	{
		if (callback_a)
		{
			node.background ([callback_a] () {
				callback_a (boost::system::errc::make_error_code (boost::system::errc::not_supported), 0);
			});
		}

		node.stats->inc (nano::stat::type::drop, detail, nano::stat::dir::out);
	}
}

/**
 * The send function behaves like a null device, it throws the data away and returns success.
*/
void nano::transport::fake::channel::send_buffer (nano::shared_const_buffer const & buffer_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::buffer_drop_policy drop_policy_a)
{
	//auto bytes = buffer_a.to_bytes ();
	auto size = buffer_a.size ();
	if (callback_a)
	{
		node.background ([callback_a, size] () {
			callback_a (boost::system::errc::make_error_code (boost::system::errc::success), size);
		});
	}
}

std::size_t nano::transport::fake::channel::hash_code () const
{
	std::hash<::nano::endpoint> hash;
	return hash (endpoint);
}

bool nano::transport::fake::channel::operator== (nano::transport::channel const & other_a) const
{
	return endpoint == other_a.get_endpoint ();
}

bool nano::transport::fake::channel::operator== (nano::transport::fake::channel const & other_a) const
{
	return endpoint == other_a.get_endpoint ();
}

std::string nano::transport::fake::channel::to_string () const
{
	return boost::str (boost::format ("%1%") % endpoint);
}

void nano::transport::fake::channel::set_peering_endpoint (nano::endpoint endpoint)
{
	peering_endpoint = endpoint;
}

nano::endpoint nano::transport::fake::channel::get_peering_endpoint () const
{
	if (peering_endpoint)
	{
		return *peering_endpoint;
	}
	else
	{
		return get_endpoint ();
	}
}
