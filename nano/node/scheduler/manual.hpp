#pragma once
#include "nano/lib/utility.hpp"

#include <nano/lib/locks.hpp>
#include <nano/lib/numbers.hpp>
#include <nano/node/election_behavior.hpp>

#include <boost/optional.hpp>

#include <deque>
#include <memory>
#include <thread>

namespace nano
{
class block;
class node;
class container_info_component;
}

namespace nano::scheduler
{
class manual final
{
	std::deque<std::tuple<std::shared_ptr<nano::block>, boost::optional<nano::uint128_t>, nano::election_behavior>> queue;
	nano::node & node;
	mutable nano::mutex mutex;
	nano::condition_variable condition;
	bool stopped{ false };
	std::thread thread;
	void notify ();
	bool predicate () const;
	void run ();

public:
	manual (nano::node & node);
	~manual ();

	void start ();
	void stop ();

	// Manualy start an election for a block
	// Call action with confirmed block, may be different than what we started with
	void push (std::shared_ptr<nano::block> const &, boost::optional<nano::uint128_t> const & = boost::none, nano::election_behavior = nano::election_behavior::normal);

	std::unique_ptr<container_info_component> collect_container_info (std::string const & name) const;
}; // class manual
} // nano::scheduler
