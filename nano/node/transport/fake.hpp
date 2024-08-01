#pragma once

#include "nano/lib/rsnano.hpp"

#include <nano/node/transport/channel.hpp>
#include <nano/node/transport/transport.hpp>

namespace nano
{
namespace transport
{
	/**
	 * Fake channel that connects to nothing and allows its attributes to be manipulated. Mostly useful for unit tests.
	 **/
	namespace fake
	{
		class channel final : public nano::transport::channel
		{
		public:
			explicit channel (nano::node &);
			explicit channel (rsnano::ChannelHandle * handle);

			std::string to_string () const override;

			uint8_t get_network_version () const override
			{
				return rsnano::rsn_channel_fake_network_version (handle);
			}

			nano::endpoint get_remote_endpoint () const override;

			nano::tcp_endpoint get_tcp_remote_endpoint () const override
			{
				return nano::transport::map_endpoint_to_tcp (get_remote_endpoint ());
			}

			nano::transport::transport_type get_type () const override
			{
				return nano::transport::transport_type::fake;
			}
		};
	} // namespace fake
} // namespace transport
} // namespace nano
