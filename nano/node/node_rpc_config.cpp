#include <nano/lib/jsonconfig.hpp>
#include <nano/lib/tomlconfig.hpp>
#include <nano/node/node_rpc_config.hpp>

#include <boost/property_tree/ptree.hpp>

nano::node_rpc_config::node_rpc_config ()
{
	rsnano::NodeRpcConfigDto dto;
	if (rsnano::rsn_node_rpc_config_create (&dto) < 0)
		throw std::runtime_error ("could not create node rpc config");
	load_dto (dto);
}

void nano::node_rpc_config::load_dto (rsnano::NodeRpcConfigDto & dto)
{
	enable_sign_hash = dto.enable_sign_hash;
	child_process.enable = dto.enable_child_process;
	child_process.rpc_path = std::string (reinterpret_cast<const char *> (dto.rpc_path), dto.rpc_path_length);
}

rsnano::NodeRpcConfigDto nano::node_rpc_config::to_dto () const
{
	rsnano::NodeRpcConfigDto dto;
	dto.enable_sign_hash = enable_sign_hash;
	dto.enable_child_process = child_process.enable;
	std::copy (child_process.rpc_path.begin (), child_process.rpc_path.end (), std::begin (dto.rpc_path));
	dto.rpc_path_length = child_process.rpc_path.size ();
	return dto;
}

nano::error nano::node_rpc_config::deserialize_toml (nano::tomlconfig & toml)
{
	toml.get_optional ("enable_sign_hash", enable_sign_hash);
	toml.get_optional<bool> ("enable_sign_hash", enable_sign_hash);

	auto child_process_l (toml.get_optional_child ("child_process"));
	if (child_process_l)
	{
		child_process_l->get_optional<bool> ("enable", child_process.enable);
		child_process_l->get_optional<std::string> ("rpc_path", child_process.rpc_path);
	}

	return toml.get_error ();
}

void nano::node_rpc_config::set_request_callback (std::function<void (boost::property_tree::ptree const &)> callback_a)
{
	request_callback = std::move (callback_a);
}
