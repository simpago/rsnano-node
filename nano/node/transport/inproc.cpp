#include <nano/node/node.hpp>
#include <nano/node/transport/inproc.hpp>

#include <boost/format.hpp>

nano::transport::inproc::channel::channel (nano::node & node_a, nano::node & destination) :
	transport::channel{ rsnano::rsn_channel_inproc_create (std::chrono::steady_clock::now ().time_since_epoch ().count ()) },
	stats (*node_a.stats),
	logger (*node_a.logger),
	limiter (node_a.network->limiter),
	io_ctx (node_a.io_ctx),
	network_packet_logging (node_a.config->logging.network_packet_logging ()),
	node{ node_a },
	destination{ destination },
	endpoint{ node_a.network->endpoint () }
{
	set_node_id (node.node_id.pub);
	set_network_version (node.network_params.network.protocol_version);
}

std::size_t nano::transport::inproc::channel::hash_code () const
{
	std::hash<::nano::endpoint> hash;
	return hash (endpoint);
}

bool nano::transport::inproc::channel::operator== (nano::transport::channel const & other_a) const
{
	return endpoint == other_a.get_endpoint ();
}

/**
 *  This function is called for every message received by the inproc channel.
 *  Note that it is called from inside the context of nano::transport::inproc::channel::send_buffer
 */
class message_visitor_inbound : public nano::message_visitor
{
public:
	message_visitor_inbound (decltype (nano::network::inbound) & inbound, std::shared_ptr<nano::transport::inproc::channel> channel) :
		inbound{ inbound },
		channel{ channel }
	{
	}

	decltype (nano::network::inbound) & inbound;

	// the channel to reply to, if a reply is generated
	std::shared_ptr<nano::transport::inproc::channel> channel;

	void default_handler (nano::message const & message) override
	{
		inbound (message, channel);
	}
};

void nano::transport::inproc::channel::send (nano::message & message_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::buffer_drop_policy drop_policy_a)
{
	auto buffer (message_a.to_shared_const_buffer ());
	auto detail = nano::message_type_to_stat_detail (message_a.get_header ().get_type ());
	auto is_droppable_by_limiter = drop_policy_a == nano::buffer_drop_policy::limiter;
	auto should_drop (limiter.should_drop (buffer.size ()));
	if (!is_droppable_by_limiter || !should_drop)
	{
		send_buffer (buffer, callback_a, drop_policy_a);
		stats.inc (nano::stat::type::message, detail, nano::stat::dir::out);
	}
	else
	{
		if (callback_a)
		{
			io_ctx.post ([callback_a] () {
				callback_a (boost::system::errc::make_error_code (boost::system::errc::not_supported), 0);
			});
		}

		stats.inc (nano::stat::type::drop, detail, nano::stat::dir::out);
		if (network_packet_logging)
		{
			logger.always_log (boost::str (boost::format ("%1% of size %2% dropped") % stats.detail_to_string (detail) % buffer.size ()));
		}
	}
}

/**
 * Send the buffer to the peer and call the callback function when done. The call never fails.
 * Note that the inbound message visitor will be called before the callback because it is called directly whereas the callback is spawned in the background.
 */
void nano::transport::inproc::channel::send_buffer (nano::shared_const_buffer const & buffer_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a, nano::buffer_drop_policy drop_policy_a)
{
	// we create a temporary channel for the reply path, in case the receiver of the message wants to reply
	auto remote_channel = std::make_shared<nano::transport::inproc::channel> (destination, node);

	// create an inbound message visitor class to handle incoming messages because that's what the message parser expects
	message_visitor_inbound visitor{ destination.network->inbound, remote_channel };

	nano::message_parser parser{ *destination.network->publish_filter, destination.block_uniquer, destination.vote_uniquer, visitor, destination.work, destination.network_params.network };

	// parse the message and action any work that needs to be done on that object via the visitor object
	auto bytes = buffer_a.to_bytes ();
	auto size = bytes.size ();
	parser.deserialize_buffer (bytes.data (), size);

	if (callback_a)
	{
		node.background ([callback_a, size] () {
			callback_a (boost::system::errc::make_error_code (boost::system::errc::success), size);
		});
	}
}

std::string nano::transport::inproc::channel::to_string () const
{
	return boost::str (boost::format ("%1%") % endpoint);
}

void nano::transport::inproc::channel::set_peering_endpoint (nano::endpoint endpoint)
{
	peering_endpoint = endpoint;
}

nano::endpoint nano::transport::inproc::channel::get_peering_endpoint () const
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