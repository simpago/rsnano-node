#include <nano/boost/asio/ip/tcp.hpp>

namespace rsnano
{
class PeerExclusionHandle;
}

namespace nano
{
class container_info_component;
using tcp_endpoint = boost::asio::ip::tcp::endpoint;

class peer_exclusion final
{
public:
	peer_exclusion ();
	peer_exclusion (nano::peer_exclusion const &) = delete;
	~peer_exclusion ();
	uint64_t add (nano::tcp_endpoint const &, std::size_t const);
	bool check (nano::tcp_endpoint const &);
	void remove (nano::tcp_endpoint const &);
	bool contains (nano::tcp_endpoint const &);
	std::size_t size () const;

private:
	rsnano::PeerExclusionHandle * handle;
};
std::unique_ptr<container_info_component> collect_container_info (peer_exclusion const & excluded_peers, std::string const & name);
}
