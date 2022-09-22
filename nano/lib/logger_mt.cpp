#include <nano/lib/logger_mt.hpp>
#include <nano/lib/rsnano.hpp>

rsnano::LoggerHandle * nano::to_logger_handle (std::shared_ptr<nano::logger_mt> const & logger_a)
{
	return rsnano::rsn_logger_create (new std::shared_ptr<nano::logger_mt> (logger_a));
}