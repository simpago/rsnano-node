#pragma once

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

			std::string to_string () const override;
			std::size_t hash_code () const override;

			void send (nano::message & message_a, std::function<void (boost::system::error_code const &, std::size_t)> const & callback_a = nullptr, nano::buffer_drop_policy policy_a = nano::buffer_drop_policy::limiter) override;

			// clang-format off
			void send_buffer (
				nano::shared_const_buffer const &,
				std::function<void (boost::system::error_code const &, std::size_t)> const & = nullptr,
				nano::buffer_drop_policy = nano::buffer_drop_policy::limiter
			) override;
			// clang-format on

			bool operator== (nano::transport::channel const &) const override;
			bool operator== (nano::transport::fake::channel const & other_a) const;

			uint8_t get_network_version () const override
			{
				return network_version;
			}

			void set_network_version (uint8_t network_version_a) override
			{
				network_version = network_version_a;
			}

			void set_endpoint (nano::endpoint const & endpoint_a)
			{
				endpoint = endpoint_a;
			}

			nano::endpoint get_endpoint () const override
			{
				return endpoint;
			}

			nano::tcp_endpoint get_tcp_endpoint () const override
			{
				return nano::transport::map_endpoint_to_tcp (endpoint);
			}

			nano::transport::transport_type get_type () const override
			{
				return nano::transport::transport_type::fake;
			}

			nano::endpoint get_peering_endpoint () const override;
			void set_peering_endpoint (nano::endpoint endpoint) override;

			void disconnect ()
			{
				endpoint = nano::endpoint (boost::asio::ip::address_v6::any (), 0);
			}

		private:
			nano::node & node;
			std::atomic<uint8_t> network_version{ 0 };
			std::optional<nano::endpoint> peering_endpoint{};
			nano::endpoint endpoint;
		};
	} // namespace fake
} // namespace transport
} // namespace nano
