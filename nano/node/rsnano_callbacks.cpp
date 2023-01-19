#include <nano/lib/config.hpp>
#include <nano/lib/logger_mt.hpp>
#include <nano/lib/rsnano.hpp>
#include <nano/lib/rsnanoutils.hpp>
#include <nano/lib/stream.hpp>
#include <nano/lib/threading.hpp>
#include <nano/lib/tomlconfig.hpp>
#include <nano/node/blockprocessor.hpp>
#include <nano/node/bootstrap/bootstrap.hpp>
#include <nano/node/election_scheduler.hpp>
#include <nano/node/lmdb/lmdb_txn.hpp>
#include <nano/node/node_observers.hpp>
#include <nano/node/rsnano_callbacks.hpp>
#include <nano/node/transport/tcp.hpp>
#include <nano/node/transport/tcp_server.hpp>
#include <nano/node/websocket.hpp>

#include <boost/property_tree/json_parser.hpp>

#include <iostream>

int32_t write_u8 (void * stream, const uint8_t value)
{
	auto s{ static_cast<nano::stream *> (stream) };
	try
	{
		nano::write<uint8_t> (*s, value);
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

int32_t write_bytes (void * stream, const uint8_t * value, size_t len)
{
	auto s{ static_cast<nano::stream *> (stream) };
	try
	{
		nano::write_bytes_raw (*s, value, len);
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

int32_t read_u8 (void * stream, uint8_t * value)
{
	auto s{ static_cast<nano::stream *> (stream) };
	try
	{
		nano::read<uint8_t> (*s, *value);
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

int32_t read_bytes (void * stream, uint8_t * buffer, size_t len)
{
	auto s{ static_cast<nano::stream *> (stream) };
	try
	{
		if (nano::try_read_raw (*s, buffer, len) != 0)
		{
			return -1;
		}
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

size_t in_avail (void * stream, int32_t * error)
{
	auto s{ static_cast<nano::stream *> (stream) };
	try
	{
		*error = 0;
		return s->in_avail ();
	}
	catch (...)
	{
		*error = 1;
		return 0;
	}
}

void ptree_put_string (void * ptree, const char * path, uintptr_t path_len, const char * value, uintptr_t value_len)
{
	auto tree (static_cast<boost::property_tree::ptree *> (ptree));
	std::string path_str (path, path_len);
	std::string value_str (value, value_len);
	tree->put (path_str, value_str);
}

void ptree_put_u64 (void * ptree, const char * path, uintptr_t path_len, uint64_t value)
{
	auto tree (static_cast<boost::property_tree::ptree *> (ptree));
	std::string path_str (path, path_len);
	tree->put (path_str, value);
}

void ptree_add (void * ptree, const char * path, uintptr_t path_len, const char * value, uintptr_t value_len)
{
	auto tree (static_cast<boost::property_tree::ptree *> (ptree));
	std::string path_str (path, path_len);
	std::string value_str (value, value_len);
	tree->add (path_str, value_str);
}

int32_t ptree_get_string (const void * ptree, const char * path, uintptr_t path_len, char * result, uintptr_t result_size)
{
	try
	{
		auto tree (static_cast<const boost::property_tree::ptree *> (ptree));
		std::string path_str (path, path_len);
		auto value (tree->get<std::string> (path_str));
		if (value.length () >= result_size)
		{
			return -1;
		}

		strcpy (result, value.c_str ());
		return value.length ();
	}
	catch (...)
	{
		return -1;
	}
}

void * ptree_create ()
{
	return new boost::property_tree::ptree ();
}

void ptree_destroy (void * handle)
{
	delete static_cast<boost::property_tree::ptree *> (handle);
}

void ptree_push_back (void * parent_handle, const char * name, const void * child_handle)
{
	auto parent_tree{ static_cast<boost::property_tree::ptree *> (parent_handle) };
	auto child_tree{ static_cast<const boost::property_tree::ptree *> (child_handle) };
	std::string name_l (name);
	parent_tree->push_back (std::make_pair (name_l, *child_tree));
}

void ptree_add_child (void * parent_handle, const char * name, const void * child_handle)
{
	auto parent_tree{ static_cast<boost::property_tree::ptree *> (parent_handle) };
	auto child_tree{ static_cast<const boost::property_tree::ptree *> (child_handle) };
	std::string name_l (name);
	parent_tree->add_child (name_l, *child_tree);
}

void ptree_put_child (void * parent_handle, const char * name, const void * child_handle)
{
	auto parent_tree{ static_cast<boost::property_tree::ptree *> (parent_handle) };
	auto child_tree{ static_cast<const boost::property_tree::ptree *> (child_handle) };
	std::string name_l (name);
	parent_tree->put_child (name_l, *child_tree);
}

void ptree_clear (void * handle)
{
	auto tree{ static_cast<boost::property_tree::ptree *> (handle) };
	tree->clear ();
}

void * ptree_to_json (void * handle)
{
	auto tree{ static_cast<boost::property_tree::ptree *> (handle) };

	std::ostringstream sstr;
	boost::property_tree::write_json (sstr, *tree);
	return new std::string (sstr.str ());
}

const char * string_chars (void * handle)
{
	auto s{ static_cast<std::string *> (handle) };
	return s->c_str ();
}

void string_delete (void * handle)
{
	auto s{ static_cast<std::string *> (handle) };
	delete s;
}

int32_t toml_put_u64 (void * toml_a, const uint8_t * key_a, uintptr_t key_len_a, uint64_t value, const uint8_t * documentation_a, uintptr_t documentation_len_a)
{
	try
	{
		auto toml{ static_cast<nano::tomlconfig *> (toml_a) };
		std::string key (reinterpret_cast<const char *> (key_a), key_len_a);
		std::string documentation (reinterpret_cast<const char *> (documentation_a), documentation_len_a);
		toml->put (key, value, documentation.c_str ());
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

int32_t toml_put_i64 (void * toml_a, const uint8_t * key_a, uintptr_t key_len_a, int64_t value, const uint8_t * documentation_a, uintptr_t documentation_len_a)
{
	try
	{
		auto toml{ static_cast<nano::tomlconfig *> (toml_a) };
		std::string key (reinterpret_cast<const char *> (key_a), key_len_a);
		std::string documentation (reinterpret_cast<const char *> (documentation_a), documentation_len_a);
		toml->put (key, value, documentation.c_str ());
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

int32_t toml_put_f64 (void * toml_a, const uint8_t * key_a, uintptr_t key_len_a, double value, const uint8_t * documentation_a, uintptr_t documentation_len_a)
{
	try
	{
		auto toml{ static_cast<nano::tomlconfig *> (toml_a) };
		std::string key (reinterpret_cast<const char *> (key_a), key_len_a);
		std::string documentation (reinterpret_cast<const char *> (documentation_a), documentation_len_a);
		toml->put (key, value, documentation.c_str ());
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

int32_t toml_put_str (void * toml_a, const uint8_t * key_a, uintptr_t key_len_a, const uint8_t * value_a, uintptr_t value_len_a, const uint8_t * documentation_a, uintptr_t documentation_len_a)
{
	try
	{
		auto toml{ static_cast<nano::tomlconfig *> (toml_a) };
		std::string key (reinterpret_cast<const char *> (key_a), key_len_a);
		std::string documentation (reinterpret_cast<const char *> (documentation_a), documentation_len_a);
		std::string value (reinterpret_cast<const char *> (value_a), value_len_a);
		toml->put (key, value, documentation.c_str ());
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

int32_t toml_put_bool (void * toml_a, const uint8_t * key_a, uintptr_t key_len_a, bool value, const uint8_t * documentation_a, uintptr_t documentation_len_a)
{
	try
	{
		auto toml{ static_cast<nano::tomlconfig *> (toml_a) };
		std::string key (reinterpret_cast<const char *> (key_a), key_len_a);
		std::string documentation (reinterpret_cast<const char *> (documentation_a), documentation_len_a);
		toml->put (key, value, documentation.c_str ());
	}
	catch (...)
	{
		return -1;
	}

	return 0;
}

struct TomlArrayHandle
{
	TomlArrayHandle (std::shared_ptr<cpptoml::array> ptr_a) :
		ptr{ ptr_a }
	{
	}
	std::shared_ptr<cpptoml::array> ptr;
};

void * toml_create_array (void * toml_a, const uint8_t * key_a, uintptr_t key_len_a, const uint8_t * documentation_a, uintptr_t documentation_len_a)
{
	try
	{
		auto toml{ static_cast<nano::tomlconfig *> (toml_a) };
		std::string key (reinterpret_cast<const char *> (key_a), key_len_a);
		std::string documentation (reinterpret_cast<const char *> (documentation_a), documentation_len_a);
		auto arr{ toml->create_array (key, documentation.c_str ()) };
		auto result = new TomlArrayHandle (arr);
		return result;
	}
	catch (...)
	{
		return nullptr;
	}
}

void toml_drop_array (void * handle_a)
{
	auto handle = reinterpret_cast<TomlArrayHandle *> (handle_a);
	delete handle;
}

void toml_array_put_str (void * handle_a, const uint8_t * value_a, uintptr_t value_len)
{
	auto handle = reinterpret_cast<TomlArrayHandle *> (handle_a);
	std::string val (reinterpret_cast<const char *> (value_a), value_len);
	handle->ptr->push_back (val);
}

void * toml_create_config ()
{
	return new nano::tomlconfig ();
}

void toml_drop_config (void * handle)
{
	delete static_cast<nano::tomlconfig *> (handle);
}

void toml_put_child (void * handle_a, const uint8_t * key_a, uintptr_t key_len_a, void * child_a)
{
	auto parent = static_cast<nano::tomlconfig *> (handle_a);
	auto child = static_cast<nano::tomlconfig *> (child_a);
	std::string key (reinterpret_cast<const char *> (key_a), key_len_a);
	parent->put_child (key, *child);
}

bool logger_try_log (void * handle_a, const uint8_t * message_a, size_t len_a)
{
	auto logger = static_cast<std::shared_ptr<nano::logger_mt> *> (handle_a);
	auto message_string = std::string (reinterpret_cast<const char *> (message_a), len_a);
	return (*logger)->try_log (message_string);
}

void logger_always_log (void * handle_a, const uint8_t * message_a, size_t len_a)
{
	auto logger = static_cast<std::shared_ptr<nano::logger_mt> *> (handle_a);
	auto message_string = std::string (reinterpret_cast<const char *> (message_a), len_a);
	return (*logger)->always_log (message_string);
}

bool listener_broadcast (void * handle_a, rsnano::MessageDto const * message_a)
{
	try
	{
		auto ptree = static_cast<boost::property_tree::ptree const *> (message_a->contents);
		nano::websocket::message message_l (static_cast<nano::websocket::topic> (message_a->topic));
		message_l.contents = *ptree;

		auto listener = static_cast<nano::websocket::listener *> (handle_a);
		listener->broadcast (message_l);
		return true;
	}
	catch (...)
	{
		return false;
	}
}

void blockprocessor_add (void * handle_a, rsnano::UncheckedInfoHandle * info_a)
{
	auto processor = static_cast<nano::block_processor *> (handle_a);
	nano::unchecked_info info{ info_a };
	processor->add (info);
}

void bootstrap_initiator_clear_pulls (void * handle_a, uint64_t bootstrap_id_a)
{
	auto bootstrap_initiator{ static_cast<nano::bootstrap_initiator *> (handle_a) };
	bootstrap_initiator->clear_pulls (bootstrap_id_a);
}

class void_fn_callback_wrapper
{
public:
	void_fn_callback_wrapper (rsnano::VoidFnCallbackHandle * callback_a) :
		callback_m{ callback_a }
	{
	}

	void_fn_callback_wrapper (void_fn_callback_wrapper const &) = delete;

	~void_fn_callback_wrapper ()
	{
		rsnano::rsn_void_fn_callback_destroy (callback_m);
	}

	void execute ()
	{
		rsnano::rsn_void_fn_callback_call (callback_m);
	}

private:
	rsnano::VoidFnCallbackHandle * callback_m;
};

void io_ctx_post (void * handle_a, rsnano::VoidFnCallbackHandle * callback_a)
{
	auto io_ctx{ static_cast<boost::asio::io_context *> (handle_a) };
	auto callback_wrapper{ std::make_shared<void_fn_callback_wrapper> (callback_a) };
	io_ctx->post ([callback_wrapper] () {
		callback_wrapper->execute ();
	});
}

void add_timed_task (void * handle_a, uint64_t delay_ms, rsnano::VoidFnCallbackHandle * callback_a)
{
	auto callback_wrapper{ std::make_shared<void_fn_callback_wrapper> (callback_a) };
	auto pool{ static_cast<std::shared_ptr<nano::thread_pool> *> (handle_a) };
	(*pool)->add_timed_task (std::chrono::steady_clock::now () + std::chrono::milliseconds (delay_ms), [callback_wrapper] () {
		callback_wrapper->execute ();
	});
}

void destroy_thread_pool (void * handle_a)
{
	auto ptr = static_cast<std::shared_ptr<nano::thread_pool> *> (handle_a);
	delete ptr;
}

void logger_destroy (void * handle_a)
{
	auto logger = static_cast<std::shared_ptr<nano::logger_mt> *> (handle_a);
	delete logger;
}

class async_connect_callback_wrapper
{
public:
	async_connect_callback_wrapper (rsnano::AsyncConnectCallbackHandle * callback_a) :
		callback_m{ callback_a }
	{
	}

	async_connect_callback_wrapper (async_connect_callback_wrapper const &) = delete;

	~async_connect_callback_wrapper ()
	{
		rsnano::rsn_async_connect_callback_destroy (callback_m);
	}

	void execute (const boost::system::error_code & ec)
	{
		auto ec_dto{ rsnano::error_code_to_dto (ec) };
		rsnano::rsn_async_connect_callback_execute (callback_m, &ec_dto);
	}

private:
	rsnano::AsyncConnectCallbackHandle * callback_m;
};

void tcp_socket_async_connect (void * handle_a, rsnano::EndpointDto const * endpoint_a, rsnano::AsyncConnectCallbackHandle * callback_a)
{
	auto callback_wrapper = std::make_shared<async_connect_callback_wrapper> (callback_a);
	auto endpoint{ rsnano::dto_to_endpoint (*endpoint_a) };
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	(*socket)->async_connect (endpoint, [callback = std::move (callback_wrapper)] (const boost::system::error_code & ec) {
		callback->execute (ec);
	});
}

class async_read_callback_wrapper
{
public:
	async_read_callback_wrapper (rsnano::AsyncReadCallbackHandle * callback_a) :
		callback_m{ callback_a }
	{
	}

	async_read_callback_wrapper (async_read_callback_wrapper const &) = delete;

	~async_read_callback_wrapper ()
	{
		rsnano::rsn_async_read_callback_destroy (callback_m);
	}

	void execute (const boost::system::error_code & ec, std::size_t size)
	{
		auto ec_dto{ rsnano::error_code_to_dto (ec) };
		rsnano::rsn_async_read_callback_execute (callback_m, &ec_dto, size);
	}

private:
	rsnano::AsyncReadCallbackHandle * callback_m;
};

void tcp_socket_async_read (void * handle_a, void * buffer_a, std::size_t size_a, rsnano::AsyncReadCallbackHandle * callback_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	auto buffer{ static_cast<std::shared_ptr<std::vector<uint8_t>> *> (buffer_a) };
	auto buffer_copy = *buffer;
	auto callback_wrapper{ std::make_shared<async_read_callback_wrapper> (callback_a) };
	(*socket)->async_read (buffer_copy, size_a, [callback_wrapper] (const boost::system::error_code & ec, std::size_t size) {
		callback_wrapper->execute (ec, size);
	});
}

void tcp_socket_async_read2 (void * handle_a, rsnano::BufferHandle * buffer_a, std::size_t size_a, rsnano::AsyncReadCallbackHandle * callback_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	auto callback_wrapper{ std::make_shared<async_read_callback_wrapper> (callback_a) };
	auto buffer{ std::make_shared<nano::buffer_wrapper> (buffer_a) };
	(*socket)->async_read (buffer, size_a, [callback_wrapper] (const boost::system::error_code & ec, std::size_t size) {
		callback_wrapper->execute (ec, size);
	});
}

class async_write_callback_wrapper
{
public:
	async_write_callback_wrapper (rsnano::AsyncWriteCallbackHandle * callback_a) :
		callback_m{ callback_a }
	{
	}

	async_write_callback_wrapper (async_write_callback_wrapper const &) = delete;

	~async_write_callback_wrapper ()
	{
		rsnano::rsn_async_write_callback_destroy (callback_m);
	}

	void execute (const boost::system::error_code & ec, std::size_t size)
	{
		auto ec_dto{ rsnano::error_code_to_dto (ec) };
		rsnano::rsn_async_write_callback_execute (callback_m, &ec_dto, size);
	}

private:
	rsnano::AsyncWriteCallbackHandle * callback_m;
};

void tcp_socket_async_write (void * handle_a, const uint8_t * buffer_a, std::size_t len_a, rsnano::AsyncWriteCallbackHandle * callback_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	nano::shared_const_buffer buffer{ buffer_a, len_a };
	auto callback_wrapper{ std::make_shared<async_write_callback_wrapper> (callback_a) };
	(*socket)->async_write (buffer, [callback_wrapper] (const boost::system::error_code & ec, std::size_t size) {
		callback_wrapper->execute (ec, size);
	});
}

void tcp_socket_remote_endpoint (void * handle_a, rsnano::EndpointDto * endpoint_a, rsnano::ErrorCodeDto * ec_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	boost::system::error_code ec;
	auto endpoint{ (*socket)->remote_endpoint (ec) };
	*endpoint_a = rsnano::endpoint_to_dto (endpoint);
	*ec_a = rsnano::error_code_to_dto (ec);
}

void tcp_socket_dispatch (void * handle_a, rsnano::VoidFnCallbackHandle * callback_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	auto callback_wrapper{ std::make_shared<void_fn_callback_wrapper> (callback_a) };
	(*socket)->dispatch ([callback_wrapper] () {
		callback_wrapper->execute ();
	});
}

void tcp_socket_post (void * handle_a, rsnano::VoidFnCallbackHandle * callback_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	auto callback_wrapper{ std::make_shared<void_fn_callback_wrapper> (callback_a) };
	(*socket)->post ([callback_wrapper] () {
		callback_wrapper->execute ();
	});
}

void tcp_socket_close (void * handle_a, rsnano::ErrorCodeDto * ec_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	boost::system::error_code ec;
	(*socket)->close (ec);
	*ec_a = rsnano::error_code_to_dto (ec);
}

void tcp_socket_local_endpoint (void * handle_a, rsnano::EndpointDto * endpoint_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	auto ep{ (*socket)->tcp_socket.local_endpoint () };
	(*endpoint_a) = rsnano::endpoint_to_dto (ep);
}

bool tcp_socket_is_open (void * handle_a)
{
	auto socket{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	return (*socket)->tcp_socket.is_open ();
}

void tcp_socket_connected (void * handle_a, rsnano::SocketHandle * socket_a)
{
	auto callback{ static_cast<std::shared_ptr<nano::node_observers> *> (handle_a) };
	(*callback)->socket_connected.notify (std::make_shared<nano::socket> (socket_a));
}

void tcp_socket_delete_callback (void * handle_a)
{
	auto callback{ static_cast<std::shared_ptr<nano::node_observers> *> (handle_a) };
	delete callback;
};
void tcp_socket_destroy (void * handle_a)
{
	auto ptr{ static_cast<std::shared_ptr<nano::tcp_socket_facade> *> (handle_a) };
	delete ptr;
}

void buffer_destroy (void * handle_a)
{
	auto ptr{ static_cast<std::shared_ptr<std::vector<uint8_t>> *> (handle_a) };
	delete ptr;
}

std::size_t buffer_size (void * handle_a)
{
	auto ptr{ static_cast<std::shared_ptr<std::vector<uint8_t>> *> (handle_a) };
	return (*ptr)->size ();
}

void message_visitor_visit (void * handle_a, rsnano::MessageHandle * msg_a, uint8_t msg_type_a)
{
	auto msg_type = static_cast<nano::message_type> (msg_type_a);
	auto visitor = static_cast<std::shared_ptr<nano::message_visitor> *> (handle_a);
	try
	{
		switch (msg_type)
		{
			case nano::message_type::keepalive:
				(*visitor)->keepalive (nano::keepalive (msg_a));
				break;
			case nano::message_type::publish:
				(*visitor)->publish (nano::publish (msg_a));
				break;
			case nano::message_type::confirm_req:
				(*visitor)->confirm_req (nano::confirm_req (msg_a));
				break;
			case nano::message_type::confirm_ack:
				(*visitor)->confirm_ack (nano::confirm_ack (msg_a));
				break;
			case nano::message_type::bulk_pull:
				(*visitor)->bulk_pull (nano::bulk_pull (msg_a));
				break;
			case nano::message_type::bulk_push:
				(*visitor)->bulk_push (nano::bulk_push (msg_a));
				break;
			case nano::message_type::frontier_req:
				(*visitor)->frontier_req (nano::frontier_req (msg_a));
				break;
			case nano::message_type::node_id_handshake:
				(*visitor)->node_id_handshake (nano::node_id_handshake (msg_a));
				break;
			case nano::message_type::bulk_pull_account:
				(*visitor)->bulk_pull_account (nano::bulk_pull_account (msg_a));
				break;
			case nano::message_type::telemetry_req:
				(*visitor)->telemetry_req (nano::telemetry_req (msg_a));
				break;
			case nano::message_type::telemetry_ack:
				(*visitor)->telemetry_ack (nano::telemetry_ack (msg_a));
				break;

			default:
				break;
		}
	}
	catch (std::exception & e)
	{
		std::cerr << "message vistior callback error: " << e.what () << std::endl;
	}
}

void message_visitor_destroy (void * handle_a)
{
	auto visitor = static_cast<std::shared_ptr<nano::message_visitor> *> (handle_a);
	delete visitor;
}

void bootstrap_observer_destroy (void * handle_a)
{
	auto observer = static_cast<std::shared_ptr<nano::tcp_server_observer> *> (handle_a);
	delete observer;
}

std::size_t bootstrap_observer_bootstrap_count (void * handle_a)
{
	auto observer = static_cast<std::shared_ptr<nano::tcp_server_observer> *> (handle_a);
	return (*observer)->get_bootstrap_count ();
}

void bootstrap_observer_exited (void * handle_a, uint8_t socket_type_a, uintptr_t inner_ptr_a, const rsnano::EndpointDto * endpoint_a)
{
	auto observer = static_cast<std::shared_ptr<nano::tcp_server_observer> *> (handle_a);
	auto socket_type = static_cast<nano::socket::type_t> (socket_type_a);
	auto endpoint = rsnano::dto_to_endpoint (*endpoint_a);
	(*observer)->tcp_server_exited (socket_type, inner_ptr_a, endpoint);
}

void bootstrap_observer_inc_bootstrap_count (void * handle_a)
{
	auto observer = static_cast<std::shared_ptr<nano::tcp_server_observer> *> (handle_a);
	(*observer)->inc_bootstrap_count ();
}

void bootstrap_observer_inc_realtime_count (void * handle_a)
{
	auto observer = static_cast<std::shared_ptr<nano::tcp_server_observer> *> (handle_a);
	(*observer)->inc_realtime_count ();
}

void bootstrap_observer_timeout (void * handle_a, uintptr_t inner_ptr_a)
{
	auto observer = static_cast<std::shared_ptr<nano::tcp_server_observer> *> (handle_a);
	(*observer)->tcp_server_timeout (inner_ptr_a);
}

void request_response_visitor_factory_destroy (void * handle_a)
{
	auto factory = static_cast<std::shared_ptr<nano::transport::request_response_visitor_factory> *> (handle_a);
	delete factory;
}

void * request_response_visitor_factory_bootstrap_visitor (void * factory_a, rsnano::TcpServerHandle * connection_a)
{
	auto factory = static_cast<std::shared_ptr<nano::transport::request_response_visitor_factory> *> (factory_a);
	auto connection = std::make_shared<nano::transport::tcp_server> (connection_a);
	auto visitor{ (*factory)->create_bootstrap (connection) };
	return new std::shared_ptr<nano::message_visitor> (visitor);
}

nano::transport::channel_tcp_observer & to_channel_tcp (void * handle_a)
{
	auto channel = static_cast<std::shared_ptr<nano::transport::channel_tcp_observer> *> (handle_a);
	return **channel;
}

void channel_tcp_data_sent (void * handle_a, const rsnano::EndpointDto * endpoint_a)
{
	auto endpoint{ rsnano::dto_to_endpoint (*endpoint_a) };
	to_channel_tcp (handle_a).data_sent (endpoint);
}

void channel_tcp_host_unreachable (void * handle_a)
{
	to_channel_tcp (handle_a).host_unreachable ();
}

void channel_tcp_message_dropped (void * handle_a, rsnano::MessageHandle * message_a, size_t buffer_size_a)
{
	auto message = nano::message_handle_to_message (message_a);
	to_channel_tcp (handle_a).message_dropped (*message, buffer_size_a);
}

void channel_tcp_message_sent (void * handle_a, rsnano::MessageHandle * message_a)
{
	auto message = nano::message_handle_to_message (message_a);
	to_channel_tcp (handle_a).message_sent (*message);
}

void channel_tcp_no_socket_drop (void * handle_a)
{
	to_channel_tcp (handle_a).no_socket_drop ();
}

void channel_tcp_write_drop (void * handle_a)
{
	to_channel_tcp (handle_a).write_drop ();
}

void channel_tcp_destroy (void * handle_a)
{
	auto channel = static_cast<std::shared_ptr<nano::transport::channel_tcp_observer> *> (handle_a);
	delete channel;
}

void channel_tcp_drop_weak (void * handle_a)
{
	auto observer = static_cast<std::weak_ptr<nano::transport::channel_tcp_observer> *> (handle_a);
	delete observer;
}

void * channel_tcp_clone_weak (void * handle_a)
{
	auto observer = static_cast<std::weak_ptr<nano::transport::channel_tcp_observer> *> (handle_a);
	return new std::weak_ptr<nano::transport::channel_tcp_observer> (*observer);
}

void * channel_tcp_observer_lock (void * handle_a)
{
	auto input{ static_cast<std::weak_ptr<nano::transport::channel_tcp_observer> *> (handle_a) };
	auto sp = input->lock ();
	if (sp)
	{
		return new std::shared_ptr<nano::transport::channel_tcp_observer> (sp);
	}
	return nullptr;
}

void bootstrap_client_observer_closed (void * handle_a)
{
	auto observer{ static_cast<std::shared_ptr<nano::bootstrap_client_observer> *> (handle_a) };
	(*observer)->bootstrap_client_closed ();
}

void bootstrap_client_observer_destroy (void * handle_a)
{
	auto observer{ static_cast<std::shared_ptr<nano::bootstrap_client_observer> *> (handle_a) };
	delete observer;
}

void * bootstrap_client_observer_to_weak (void * handle_a)
{
	auto observer{ static_cast<std::shared_ptr<nano::bootstrap_client_observer> *> (handle_a) };
	return new std::weak_ptr<nano::bootstrap_client_observer> (*observer);
}

void * bootstrap_client_weak_to_observer (void * handle_a)
{
	auto weak{ static_cast<std::weak_ptr<nano::bootstrap_client_observer> *> (handle_a) };
	auto observer = (*weak).lock ();
	void * result = nullptr;
	if (observer)
		result = new std::shared_ptr<nano::bootstrap_client_observer> (observer);
	return result;
}

void bootstrap_client_observer_weak_destroy (void * handle_a)
{
	auto observer{ static_cast<std::weak_ptr<nano::bootstrap_client_observer> *> (handle_a) };
	delete observer;
}

void txn_callbacks_destroy (void * handle_a)
{
	auto callbacks = static_cast<nano::mdb_txn_callbacks *> (handle_a);
	delete callbacks;
}

void txn_callbacks_start (void * handle_a, uint64_t txn_id_a, bool is_write_a)
{
	auto callbacks = static_cast<nano::mdb_txn_callbacks *> (handle_a);
	callbacks->txn_start (txn_id_a, is_write_a);
}

void txn_callbacks_end (void * handle_a, uint64_t txn_id_a)
{
	auto callbacks = static_cast<nano::mdb_txn_callbacks *> (handle_a);
	callbacks->txn_end (txn_id_a);
}

bool message_visitor_bootstrap_processed (void * handle_a)
{
	auto visitor = static_cast<std::shared_ptr<nano::message_visitor> *> (handle_a);
	return (static_cast<nano::transport::tcp_server::bootstrap_message_visitor *> (visitor->get ()))->processed;
}

void election_scheduler_activate (void * scheduler_a, const uint8_t * account_a, rsnano::TransactionHandle * txn_a)
{
	auto election_scheduler = static_cast<nano::election_scheduler *> (scheduler_a);
	nano::account account;
	std::copy (account_a, account_a + 32, std::begin (account.bytes));
	nano::transaction_wrapper txn_wrapper{ txn_a };
	election_scheduler->activate (account, txn_wrapper);
}

static bool callbacks_set = false;

void rsnano::set_rsnano_callbacks ()
{
	if (callbacks_set)
		return;

	rsnano::rsn_callback_write_u8 (write_u8);
	rsnano::rsn_callback_write_bytes (write_bytes);
	rsnano::rsn_callback_read_u8 (read_u8);
	rsnano::rsn_callback_read_bytes (read_bytes);
	rsnano::rsn_callback_in_avail (in_avail);

	rsnano::rsn_callback_property_tree_put_string (ptree_put_string);
	rsnano::rsn_callback_property_tree_put_u64 (ptree_put_u64);
	rsnano::rsn_callback_property_tree_add (ptree_add);
	rsnano::rsn_callback_property_tree_get_string (ptree_get_string);
	rsnano::rsn_callback_property_tree_create (ptree_create);
	rsnano::rsn_callback_property_tree_destroy (ptree_destroy);
	rsnano::rsn_callback_property_tree_push_back (ptree_push_back);
	rsnano::rsn_callback_property_tree_add_child (ptree_add_child);
	rsnano::rsn_callback_property_tree_put_child (ptree_put_child);
	rsnano::rsn_callback_property_tree_clear (ptree_clear);
	rsnano::rsn_callback_property_tree_to_json (ptree_to_json);

	rsnano::rsn_callback_string_chars (string_chars);
	rsnano::rsn_callback_string_delete (string_delete);

	rsnano::rsn_callback_toml_put_u64 (toml_put_u64);
	rsnano::rsn_callback_toml_put_i64 (toml_put_i64);
	rsnano::rsn_callback_toml_put_str (toml_put_str);
	rsnano::rsn_callback_toml_put_bool (toml_put_bool);
	rsnano::rsn_callback_toml_put_f64 (toml_put_f64);
	rsnano::rsn_callback_toml_create_array (toml_create_array);
	rsnano::rsn_callback_toml_array_put_str (toml_array_put_str);
	rsnano::rsn_callback_toml_create_config (toml_create_config);
	rsnano::rsn_callback_toml_drop_config (toml_drop_config);
	rsnano::rsn_callback_toml_put_child (toml_put_child);
	rsnano::rsn_callback_toml_drop_array (toml_drop_array);

	rsnano::rsn_callback_try_log (logger_try_log);
	rsnano::rsn_callback_always_log (logger_always_log);
	rsnano::rsn_callback_listener_broadcast (listener_broadcast);
	rsnano::rsn_callback_block_processor_add (blockprocessor_add);
	rsnano::rsn_callback_block_bootstrap_initiator_clear_pulls (bootstrap_initiator_clear_pulls);
	rsnano::rsn_callback_add_timed_task (add_timed_task);
	rsnano::rsn_callback_destroy_thread_pool (destroy_thread_pool);
	rsnano::rsn_callback_logger_destroy (logger_destroy);

	rsnano::rsn_callback_io_ctx_post (io_ctx_post);

	rsnano::rsn_callback_tcp_socket_async_connect (tcp_socket_async_connect);
	rsnano::rsn_callback_tcp_socket_async_read (tcp_socket_async_read);
	rsnano::rsn_callback_tcp_socket_async_read2 (tcp_socket_async_read2);
	rsnano::rsn_callback_tcp_socket_async_write (tcp_socket_async_write);
	rsnano::rsn_callback_tcp_socket_remote_endpoint (tcp_socket_remote_endpoint);
	rsnano::rsn_callback_tcp_socket_dispatch (tcp_socket_dispatch);
	rsnano::rsn_callback_tcp_socket_post (tcp_socket_post);
	rsnano::rsn_callback_tcp_socket_close (tcp_socket_close);
	rsnano::rsn_callback_tcp_socket_destroy (tcp_socket_destroy);
	rsnano::rsn_callback_tcp_socket_local_endpoint (tcp_socket_local_endpoint);
	rsnano::rsn_callback_tcp_socket_is_open (tcp_socket_is_open);
	rsnano::rsn_callback_tcp_socket_connected (tcp_socket_connected);
	rsnano::rsn_callback_delete_tcp_socket_callback (tcp_socket_delete_callback);

	rsnano::rsn_callback_channel_tcp_observer_data_sent (channel_tcp_data_sent);
	rsnano::rsn_callback_channel_tcp_observer_host_unreachable (channel_tcp_host_unreachable);
	rsnano::rsn_callback_channel_tcp_observer_message_dropped (channel_tcp_message_dropped);
	rsnano::rsn_callback_channel_tcp_observer_message_sent (channel_tcp_message_sent);
	rsnano::rsn_callback_channel_tcp_observer_no_socket_drop (channel_tcp_no_socket_drop);
	rsnano::rsn_callback_channel_tcp_observer_write_drop (channel_tcp_write_drop);
	rsnano::rsn_callback_channel_tcp_observer_destroy (channel_tcp_destroy);
	rsnano::rsn_callback_channel_tcp_observer_drop_weak (channel_tcp_drop_weak);
	rsnano::rsn_callback_channel_tcp_observer_clone_weak (channel_tcp_clone_weak);
	rsnano::rsn_callback_channel_tcp_observer_lock (channel_tcp_observer_lock);

	rsnano::rsn_callback_buffer_destroy (buffer_destroy);
	rsnano::rsn_callback_buffer_size (buffer_size);

	rsnano::rsn_callback_message_visitor_visit (message_visitor_visit);
	rsnano::rsn_callback_message_visitor_destroy (message_visitor_destroy);

	rsnano::rsn_callback_bootstrap_observer_destroy (bootstrap_observer_destroy);
	rsnano::rsn_callback_bootstrap_observer_bootstrap_count (bootstrap_observer_bootstrap_count);
	rsnano::rsn_callback_bootstrap_observer_exited (bootstrap_observer_exited);
	rsnano::rsn_callback_bootstrap_observer_inc_bootstrap_count (bootstrap_observer_inc_bootstrap_count);
	rsnano::rsn_callback_bootstrap_observer_inc_realtime_count (bootstrap_observer_inc_realtime_count);
	rsnano::rsn_callback_bootstrap_observer_timeout (bootstrap_observer_timeout);

	rsnano::rsn_callback_request_response_visitor_factory_destroy (request_response_visitor_factory_destroy);
	rsnano::rsn_callback_request_response_visitor_factory_bootstrap_visitor (request_response_visitor_factory_bootstrap_visitor);

	rsnano::rsn_callback_bootstrap_client_observer_closed (bootstrap_client_observer_closed);
	rsnano::rsn_callback_bootstrap_client_observer_destroy (bootstrap_client_observer_destroy);
	rsnano::rsn_callback_bootstrap_client_observer_to_weak (bootstrap_client_observer_to_weak);
	rsnano::rsn_callback_bootstrap_client_weak_to_observer (bootstrap_client_weak_to_observer);
	rsnano::rsn_callback_bootstrap_client_observer_weak_destroy (bootstrap_client_observer_weak_destroy);

	rsnano::rsn_callback_txn_callbacks_destroy (txn_callbacks_destroy);
	rsnano::rsn_callback_txn_callbacks_start (txn_callbacks_start);
	rsnano::rsn_callback_txn_callbacks_end (txn_callbacks_end);

	rsnano::rsn_callback_message_visitor_bootstrap_processed (message_visitor_bootstrap_processed);
	rsnano::rsn_callback_memory_intensive_instrumentation (nano::memory_intensive_instrumentation);
	rsnano::rsn_callback_is_sanitizer_build (nano::is_sanitizer_build);

	rsnano::rsn_callback_election_scheduler_activate (election_scheduler_activate);

	callbacks_set = true;
}
