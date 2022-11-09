#pragma once

#include <nano/lib/config.hpp>
#include <nano/lib/locks.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/lib/observer_set.hpp>
#include <nano/lib/utility.hpp>

#include <boost/optional.hpp>
#include <boost/thread/thread.hpp>

#include <atomic>
#include <memory>

namespace rsnano
{
class WorkPoolHandle;
class WorkTicketHandle;
}
namespace nano
{
std::string to_string (nano::work_version const version_a);

class block;
class block_details;
enum class block_type : uint8_t;

class opencl_work;
class work_item final
{
public:
	work_item (nano::work_version const version_a, nano::root const & item_a, uint64_t difficulty_a, std::function<void (boost::optional<uint64_t> const &)> const & callback_a) :
		version (version_a), item (item_a), difficulty (difficulty_a), callback (callback_a)
	{
	}
	nano::work_version const version;
	nano::root const item;
	uint64_t const difficulty;
	std::function<void (boost::optional<uint64_t> const &)> const callback;
};
class work_ticket
{
public:
	work_ticket ();
	work_ticket (rsnano::WorkTicketHandle * handle_a);
	work_ticket (work_ticket const &);
	work_ticket (work_ticket && other_a);
	~work_ticket ();
	bool expired () const;
	rsnano::WorkTicketHandle * handle;
};
class work_pool final
{
public:
	work_pool (nano::network_constants & network_constants, unsigned, std::chrono::nanoseconds = std::chrono::nanoseconds (0), std::function<boost::optional<uint64_t> (nano::work_version const, nano::root const, uint64_t, nano::work_ticket)> = nullptr);
	work_pool (work_pool const &) = delete;
	work_pool (work_pool &&) = delete;
	~work_pool ();
	void stop ();
	void cancel (nano::root const &);
	void generate (nano::work_version const, nano::root const &, uint64_t, std::function<void (boost::optional<uint64_t> const &)>);
	boost::optional<uint64_t> generate (nano::work_version const, nano::root const &, uint64_t);
	// For tests only
	boost::optional<uint64_t> generate (nano::root const &);
	boost::optional<uint64_t> generate (nano::root const &, uint64_t);
	size_t size ();
	size_t pending_size ();
	size_t pending_value_size () const;
	size_t thread_count () const;
	bool has_opencl () const;
	uint64_t threshold_base (nano::work_version const version_a) const;
	uint64_t difficulty (nano::work_version const version_a, nano::root const & root_a, uint64_t const work_a) const;

private:
	rsnano::WorkPoolHandle * handle;
};

std::unique_ptr<container_info_component> collect_container_info (work_pool & work_pool, std::string const & name);
}
