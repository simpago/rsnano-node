#include <nano/crypto_lib/random_pool.hpp>
#include <nano/lib/blocks.hpp>
#include <nano/lib/cli.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/thread_runner.hpp>
#include <nano/lib/utility.hpp>
#include <nano/nano_node/daemon.hpp>
#include <nano/node/active_elections.hpp>
#include <nano/node/cli.hpp>
#include <nano/node/daemonconfig.hpp>
#include <nano/node/inactive_node.hpp>
#include <nano/node/ipc/ipc_server.hpp>
#include <nano/node/json_handler.hpp>
#include <nano/node/node.hpp>
#include <nano/node/rsnano_callbacks.hpp>
#include <nano/node/transport/inproc.hpp>
#include <nano/secure/ledger.hpp>
#include <nano/store/pending.hpp>

#include <boost/dll/runtime_symbol_info.hpp>
#include <boost/format.hpp>
#include <boost/lexical_cast.hpp>
#include <boost/program_options.hpp>
#include <boost/range/adaptor/reversed.hpp>
#ifdef _WIN32
#ifndef NOMINMAX
#define NOMINMAX
#endif
#endif
#include <boost/stacktrace.hpp>
#include <boost/unordered_map.hpp>
#include <boost/unordered_set.hpp>

#include <numeric>
#include <sstream>

namespace
{
class uint64_from_hex // For use with boost::lexical_cast to read hexadecimal strings
{
public:
	uint64_t value;
};
std::istream & operator>> (std::istream & in, uint64_from_hex & out_val);

class address_library_pair
{
public:
	uint64_t address;
	std::string library;

	address_library_pair (uint64_t address, std::string library);
	bool operator< (const address_library_pair & other) const;
	bool operator== (const address_library_pair & other) const;
};

}

int main (int argc, char * const * argv)
{
	rsnano::set_rsnano_callbacks ();
	nano::set_umask (); // Make sure the process umask is set before any files are created
	nano::initialize_file_descriptor_limit ();

	nano::node_singleton_memory_pool_purge_guard memory_pool_cleanup_guard;

	boost::program_options::options_description description ("Command line options");
	// clang-format off
	description.add_options ()
		("help", "Print out options")
		("version", "Prints out version")
		("config", boost::program_options::value<std::vector<nano::config_key_value_pair>>()->multitoken(), "Pass node configuration values. This takes precedence over any values in the configuration file. This option can be repeated multiple times.")
		("rpcconfig", boost::program_options::value<std::vector<nano::config_key_value_pair>>()->multitoken(), "Pass rpc configuration values. This takes precedence over any values in the configuration file. This option can be repeated multiple times.")
		("daemon", "Start node daemon")
		("debug_block_count", "Display the number of blocks")
		("debug_prune", "Prune accounts up to last confirmed blocks (EXPERIMENTAL)")
		("platform", boost::program_options::value<std::string> (), "Defines the <platform> for OpenCL commands")
		("device", boost::program_options::value<std::string> (), "Defines <device> for OpenCL command")
		("threads", boost::program_options::value<std::string> (), "Defines <threads> count for various commands")
		("difficulty", boost::program_options::value<std::string> (), "Defines <difficulty> for OpenCL command, HEX")
		("multiplier", boost::program_options::value<std::string> (), "Defines <multiplier> for work generation. Overrides <difficulty>")
		("count", boost::program_options::value<std::string> (), "Defines <count> for various commands")
		("pow_sleep_interval", boost::program_options::value<std::string> (), "Defines the amount to sleep inbetween each pow calculation attempt")
		("address_column", boost::program_options::value<std::string> (), "Defines which column the addresses are located, 0 indexed")
		("silent", "Silent command execution");
	// clang-format on
	nano::add_node_options (description);
	nano::add_node_flag_options (description);
	boost::program_options::variables_map vm;
	try
	{
		boost::program_options::store (boost::program_options::parse_command_line (argc, argv, description), vm);
	}
	catch (boost::program_options::error const & err)
	{
		std::cerr << err.what () << std::endl;
		return 1;
	}
	boost::program_options::notify (vm);
	int result (0);

	if (vm.contains ("initialize") || vm.contains ("wallet_create") || vm.contains ("wallet_decrypt_unsafe") || vm.contains ("wallet_list"))
	{
		// don't log by default for these commands
		nano::logger::initialize_for_tests ();
	}
	else
	{
		nano::logger::initialize ();
	}

	auto network (vm.find ("network"));
	if (network != vm.end ())
	{
		auto err (nano::network_constants::set_active_network (network->second.as<std::string> ()));
		if (err)
		{
			std::cerr << nano::network_constants::active_network_err_msg << std::endl;
			std::exit (1);
		}
	}

	nano::network_params network_params{ nano::network_constants::active_network () };
	auto data_path_it = vm.find ("data_path");
	std::filesystem::path data_path ((data_path_it != vm.end ()) ? std::filesystem::path (data_path_it->second.as<std::string> ()) : nano::working_path ());
	auto ec = nano::handle_node_options (vm);
	if (ec == nano::error_cli::unknown_command)
	{
		if (vm.count ("daemon") > 0)
		{
			nano::daemon daemon;
			nano::node_flags flags;
			auto flags_ec = nano::update_flags (flags, vm);
			if (flags_ec)
			{
				std::cerr << flags_ec.message () << std::endl;
				std::exit (1);
			}
			daemon.run (data_path, flags);
		}
		else if (vm.count ("debug_block_count"))
		{
			auto node_flags = nano::inactive_node_flag_defaults ();
			nano::update_flags (node_flags, vm);
			auto gen_cache{ node_flags.generate_cache () };
			gen_cache.enable_block_count (true);
			node_flags.set_generate_cache (gen_cache);
			nano::inactive_node inactive_node (data_path, node_flags);
			auto node = inactive_node.node;
			std::cout << boost::str (boost::format ("Block count: %1%\n") % node->ledger.block_count ());
		}
		else if (vm.count ("debug_prune"))
		{
			auto node_flags = nano::inactive_node_flag_defaults ();
			node_flags.set_read_only (false);
			nano::update_flags (node_flags, vm);
			nano::inactive_node inactive_node (data_path, node_flags);
			auto node = inactive_node.node;
			node->ledger_pruning (node_flags.block_processor_batch_size () != 0 ? node_flags.block_processor_batch_size () : 16 * 1024, true);
		}
		else if (vm.count ("version"))
		{
			std::cout << "Version " << NANO_VERSION_STRING << "\n"
					  << "Build Info " << BUILD_INFO << std::endl;
		}
		else
		{
			// Issue #3748
			// Regardless how the options were added, output the options in alphabetical order so they are easy to find.
			boost::program_options::options_description sorted_description ("Command line options");
			nano::sort_options_description (description, sorted_description);
			std::cout << sorted_description << std::endl;
			result = -1;
		}
	}
	return result;
}

namespace
{
std::istream & operator>> (std::istream & in, uint64_from_hex & out_val)
{
	in >> std::hex >> out_val.value;
	return in;
}

address_library_pair::address_library_pair (uint64_t address, std::string library) :
	address (address), library (library)
{
}

bool address_library_pair::operator< (const address_library_pair & other) const
{
	return address < other.address;
}

bool address_library_pair::operator== (const address_library_pair & other) const
{
	return address == other.address;
}
}
