#include <nano/lib/rsnano.hpp>
#include <nano/lib/utility.hpp>
#include <nano/node/bandwidth_limiter.hpp>

/*
 * bandwidth_limiter
 */

nano::bandwidth_limiter::bandwidth_limiter (std::size_t limit_a, double burst_ratio_a) :
	handle{ rsnano::rsn_bandwidth_limiter_create (burst_ratio_a, limit_a) }
{
}

nano::bandwidth_limiter::bandwidth_limiter (rsnano::BandwidthLimiterHandle * handle_a) :
	handle{ handle_a }
{
}

nano::bandwidth_limiter::bandwidth_limiter (nano::bandwidth_limiter && other_a) :
	handle{ other_a.handle }
{
	other_a.handle = nullptr;
}

nano::bandwidth_limiter::~bandwidth_limiter ()
{
	if (handle)
		rsnano::rsn_bandwidth_limiter_destroy (handle);
}

bool nano::bandwidth_limiter::should_pass (std::size_t message_size_a)
{
	return rsnano::rsn_bandwidth_limiter_should_pass (handle, message_size_a);
}

void nano::bandwidth_limiter::reset (std::size_t limit_a, double burst_ratio_a)
{
	rsnano::rsn_bandwidth_limiter_reset (handle, burst_ratio_a, limit_a);
}

/*
 * outbound_bandwidth_limiter
 */

nano::outbound_bandwidth_limiter::outbound_bandwidth_limiter (nano::outbound_bandwidth_limiter::config config_a)
{
	rsnano::OutboundBandwidthLimiterConfigDto config_dto;
	config_dto.standard_limit = config_a.standard_limit;
	config_dto.standard_burst_ratio = config_a.standard_burst_ratio;
	config_dto.bootstrap_limit = config_a.bootstrap_limit;
	config_dto.bootstrap_burst_ratio = config_a.bootstrap_burst_ratio;
	handle = rsnano::rsn_outbound_bandwidth_limiter_create (&config_dto);
}

nano::outbound_bandwidth_limiter::~outbound_bandwidth_limiter ()
{
	rsnano::rsn_outbound_bandwidth_limiter_destroy (handle);
}

bool nano::outbound_bandwidth_limiter::should_pass (std::size_t buffer_size, nano::bandwidth_limit_type type)
{
	return rsnano::rsn_outbound_bandwidth_limiter_should_pass (handle, buffer_size, static_cast<uint8_t> (type));
}

void nano::outbound_bandwidth_limiter::reset (std::size_t limit, double burst_ratio, nano::bandwidth_limit_type type)
{
	rsnano::rsn_outbound_bandwidth_limiter_reset (handle, burst_ratio, limit, static_cast<uint8_t> (type));
}