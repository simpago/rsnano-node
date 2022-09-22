#pragma once

#include <nano/node/bootstrap/bootstrap.hpp>

#include <atomic>
#include <future>

namespace nano
{
class node;

class frontier_req_client;
class bulk_push_client;

/**
 * Polymorphic base class for bootstrap sessions.
 */
class bootstrap_attempt : public std::enable_shared_from_this<bootstrap_attempt>
{
public:
	explicit bootstrap_attempt (std::shared_ptr<nano::node> const & node_a, nano::bootstrap_mode mode_a, uint64_t incremental_id_a, std::string id_a);
	explicit bootstrap_attempt (rsnano::BootstrapAttemptHandle * handle);
	virtual ~bootstrap_attempt ();
	virtual void run () = 0;
	virtual void stop ();
	bool still_pulling ();
	void pull_started ();
	void pull_finished ();
	bool should_log ();
	std::string mode_text ();
	virtual bool process_block (std::shared_ptr<nano::block> const &, nano::account const &, uint64_t, nano::bulk_pull::count_t, bool, unsigned);
	virtual void get_information (boost::property_tree::ptree &) = 0;
	virtual void block_processed (nano::transaction const & tx, nano::process_return const & result, nano::block const & block);
	uint64_t total_blocks () const;
	void total_blocks_inc ();
	unsigned get_pulling () const;
	void inc_pulling ();
	bool get_stopped () const;
	void set_stopped ();
	bool get_started () const;
	bool set_started ();
	nano::bootstrap_mode get_mode () const;
	unsigned get_requeued_pulls () const;
	void inc_requeued_pulls ();
	bool get_frontiers_received () const;
	void set_frontiers_received (bool);
	std::chrono::seconds duration () const;

	std::string id () const;
	uint64_t get_incremental_id () const;
	void notify_all ();
	rsnano::BootstrapAttemptHandle * handle;
};
}
