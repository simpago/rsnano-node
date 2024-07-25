#include "nano/lib/rsnanoutils.hpp"

#include <nano/lib/rsnano.hpp>
#include <nano/lib/stacktrace.hpp>
#include <nano/lib/utility.hpp>

#include <boost/dll/runtime_symbol_info.hpp>
#include <boost/program_options.hpp>

#include <cstddef>
#include <fstream>
#include <iostream>
#include <limits>
#include <memory>
#include <sstream>
#include <string_view>
#include <thread>

#ifndef _WIN32
#include <sys/resource.h>
#endif

std::size_t nano::get_file_descriptor_limit ()
{
	std::size_t fd_limit = std::numeric_limits<std::size_t>::max ();
#ifndef _WIN32
	rlimit limit{};
	if (getrlimit (RLIMIT_NOFILE, &limit) == 0)
	{
		fd_limit = static_cast<std::size_t> (limit.rlim_cur);
	}
#endif
	return fd_limit;
}

void nano::set_file_descriptor_limit (std::size_t limit)
{
#ifndef _WIN32
	rlimit fd_limit{};
	if (-1 == getrlimit (RLIMIT_NOFILE, &fd_limit))
	{
		std::cerr << "WARNING: Unable to get current limits for the number of open file descriptors: " << std::strerror (errno);
		return;
	}

	if (fd_limit.rlim_cur >= limit)
	{
		return;
	}

	fd_limit.rlim_cur = std::min (static_cast<rlim_t> (limit), fd_limit.rlim_max);
	if (-1 == setrlimit (RLIMIT_NOFILE, &fd_limit))
	{
		std::cerr << "WARNING: Unable to set limits for the number of open file descriptors: " << std::strerror (errno);
		return;
	}
#endif
}

void nano::initialize_file_descriptor_limit ()
{
	nano::set_file_descriptor_limit (DEFAULT_FILE_DESCRIPTOR_LIMIT);
	auto limit = nano::get_file_descriptor_limit ();
	if (limit < DEFAULT_FILE_DESCRIPTOR_LIMIT)
	{
		std::cerr << "WARNING: Current file descriptor limit of " << limit << " is lower than the " << DEFAULT_FILE_DESCRIPTOR_LIMIT << " recommended. Node was unable to change it." << std::endl;
	}
}

nano::container_info_composite::container_info_composite (std::string const & name) :
	container_info_component{ rsnano::rsn_container_info_composite_create (name.c_str ()) }
{
}

nano::container_info_composite::container_info_composite (rsnano::ContainerInfoComponentHandle * handle_a) :
	container_info_component{ handle_a }
{
}

bool nano::container_info_composite::is_composite () const
{
	return true;
}

void nano::container_info_composite::add_component (std::unique_ptr<container_info_component> child)
{
	rsnano::rsn_container_info_composite_child_add (handle, child->handle);
}

std::vector<std::unique_ptr<nano::container_info_component>> nano::container_info_composite::get_children () const
{
	std::vector<std::unique_ptr<nano::container_info_component>> result;
	auto size = rsnano::rsn_container_info_composite_children_len (handle);
	result.reserve (size);
	for (auto i = 0; i < size; ++i)
	{
		auto child_handle = rsnano::rsn_container_info_composite_child (handle, i);
		if (rsnano::rsn_container_info_component_is_composite (child_handle))
		{
			result.push_back (std::make_unique<nano::container_info_composite> (child_handle));
		}
		else
		{
			result.push_back (std::make_unique<nano::container_info_leaf> (child_handle));
		}
	}

	return result;
}

std::string nano::container_info_composite::get_name () const
{
	rsnano::StringDto dto;
	rsnano::rsn_container_info_composite_name (handle, &dto);
	return rsnano::convert_dto_to_string (dto);
}

nano::container_info_component::container_info_component (rsnano::ContainerInfoComponentHandle * handle_a) :
	handle{ handle_a }
{
}

nano::container_info_component::container_info_component (container_info_component && other_a)
{
	if (handle)
	{
		rsnano::rsn_container_info_component_destroy (handle);
	}
	handle = other_a.handle;
	other_a.handle = nullptr;
}

nano::container_info_component::~container_info_component ()
{
	if (handle)
	{
		rsnano::rsn_container_info_component_destroy (handle);
	}
}

nano::container_info_leaf::container_info_leaf (const container_info & info) :
	container_info_component{ rsnano::rsn_container_info_leaf_create (info.name.c_str (), info.count, info.sizeof_element) },
	info (info),
	info_loaded{ true }
{
}

nano::container_info_leaf::container_info_leaf (rsnano::ContainerInfoComponentHandle * handle_a) :
	container_info_component{ handle_a },
	info_loaded{ false },
	info{}
{
}

bool nano::container_info_leaf::is_composite () const
{
	return false;
}

nano::container_info const & nano::container_info_leaf::get_info () const
{
	if (!info_loaded)
	{
		rsnano::ContainerInfoDto info_dto;
		rsnano::rsn_container_info_leaf_get_info (handle, &info_dto);
		info.count = info_dto.count;
		info.sizeof_element = info_dto.sizeof_element;
		info.name = rsnano::convert_dto_to_string (info_dto.name);
		info_loaded = true;
	}
	return info;
}

/*
 * Backing code for "release_assert" & "debug_assert", which are macros
 */
void assert_internal (char const * check_expr, char const * func, char const * file, unsigned int line, bool is_release_assert, std::string_view error_msg)
{
	std::cerr << "Assertion (" << check_expr << ") failed\n"
			  << func << "\n"
			  << file << ":" << line << "\n";
	if (!error_msg.empty ())
	{
		std::cerr << "Error: " << error_msg << "\n";
	}
	std::cerr << "\n";

	// Output stack trace to cerr
	auto backtrace_str = nano::generate_stacktrace ();
	std::cerr << backtrace_str << std::endl;

	// "abort" at the end of this function will go into any signal handlers (the daemon ones will generate a stack trace and load memory address files on non-Windows systems).
	// As there is no async-signal-safe way to generate stacktraces on Windows it must be done before aborting
#ifdef _WIN32
	{
		// Try construct the stacktrace dump in the same folder as the the running executable, otherwise use the current directory.
		boost::system::error_code err;
		auto running_executable_filepath = boost::dll::program_location (err);
		std::string filename = is_release_assert ? "nano_node_backtrace_release_assert.txt" : "nano_node_backtrace_assert.txt";
		std::string filepath = filename;
		if (!err)
		{
			filepath = (running_executable_filepath.parent_path () / filename).string ();
		}

		std::ofstream file (filepath);
		nano::set_secure_perm_file (filepath);
		file << backtrace_str;
	}
#endif

	abort ();
}

// Issue #3748
void nano::sort_options_description (const boost::program_options::options_description & source, boost::program_options::options_description & target)
{
	// Grab all of the options, get the option display name, stick it in a map using the display name as
	// the key (the map will sort) and the value as the option itself.
	const auto & options = source.options ();
	std::map<std::string, boost::shared_ptr<boost::program_options::option_description>> sorted_options;
	for (const auto & option : options)
	{
		auto pair = std::make_pair (option->canonical_display_name (2), option);
		sorted_options.insert (pair);
	}

	// Rebuild for display purposes only.
	for (const auto & option_pair : sorted_options)
	{
		target.add (option_pair.second);
	}
}
