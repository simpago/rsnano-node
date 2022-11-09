#pragma once

#include <cstddef>

namespace rsnano
{
class OutboundBandwidthLimiterHandle;
class BandwidthLimiterHandle;
}

namespace nano
{
/**
 * Enumeration for different bandwidth limits for different traffic types
 */
enum class bandwidth_limit_type
{
	/** For all message */
	standard,
	/** For bootstrap (asc_pull_ack, asc_pull_req) traffic */
	bootstrap
};

/**
 * Class that tracks and manages bandwidth limits for IO operations
 */
class bandwidth_limiter final
{
public:
	// initialize with limit 0 = unbounded
	bandwidth_limiter (std::size_t limit, double burst_ratio);
	explicit bandwidth_limiter (rsnano::BandwidthLimiterHandle * handle_a);
	bandwidth_limiter (bandwidth_limiter && other_a);
	bandwidth_limiter (bandwidth_limiter const &) = delete;
	~bandwidth_limiter ();
	bool should_pass (std::size_t buffer_size);
	void reset (std::size_t limit, double burst_ratio);

public:
	rsnano::BandwidthLimiterHandle * handle;
};

class outbound_bandwidth_limiter final
{
public: // Config
	struct config
	{
		// standard
		std::size_t standard_limit;
		double standard_burst_ratio;
		// bootstrap
		std::size_t bootstrap_limit;
		double bootstrap_burst_ratio;
	};

public:
	explicit outbound_bandwidth_limiter (config);
	outbound_bandwidth_limiter (outbound_bandwidth_limiter const &) = delete;
	outbound_bandwidth_limiter (outbound_bandwidth_limiter &&) = delete;
	~outbound_bandwidth_limiter ();
	/**
	 * Check whether packet falls withing bandwidth limits and should be allowed
	 * @return true if OK, false if needs to be dropped
	 */
	bool should_pass (std::size_t buffer_size, bandwidth_limit_type);
	/**
	 * Reset limits of selected limiter type to values passed in arguments
	 */
	void reset (std::size_t limit, double burst_ratio, bandwidth_limit_type = bandwidth_limit_type::standard);

	rsnano::OutboundBandwidthLimiterHandle * handle;
};
}