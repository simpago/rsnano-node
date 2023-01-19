if(${CMAKE_SYSTEM_NAME} MATCHES "Darwin")
  set(platform_sources plat/posix/perms.cpp plat/darwin/thread_role.cpp
                       plat/default/debugging.cpp)
elseif(${CMAKE_SYSTEM_NAME} MATCHES "Windows")
  set(platform_sources plat/windows/perms.cpp plat/windows/registry.cpp
                       plat/windows/thread_role.cpp plat/default/debugging.cpp)
elseif(${CMAKE_SYSTEM_NAME} MATCHES "Linux")
  set(platform_sources plat/posix/perms.cpp plat/linux/thread_role.cpp
                       plat/linux/debugging.cpp)
elseif(${CMAKE_SYSTEM_NAME} MATCHES "FreeBSD")
  set(platform_sources plat/posix/perms.cpp plat/freebsd/thread_role.cpp
                       plat/default/debugging.cpp)
else()
  error("Unknown platform: ${CMAKE_SYSTEM_NAME}")
endif()

add_library(
  nano_lib
  ${platform_sources}
  asio.hpp
  asio.cpp
  blockbuilders.hpp
  blockbuilders.cpp
  blocks.hpp
  blocks.cpp
  cli.hpp
  cli.cpp
  config.hpp
  config.cpp
  configbase.hpp
  diagnosticsconfig.hpp
  diagnosticsconfig.cpp
  epoch.hpp
  epoch.cpp
  errors.hpp
  errors.cpp
  ipc.hpp
  ipc.cpp
  ipc_client.hpp
  ipc_client.cpp
  json_error_response.hpp
  jsonconfig.hpp
  jsonconfig.cpp
  lmdbconfig.hpp
  lmdbconfig.cpp
  locks.hpp
  locks.cpp
  logger_mt.hpp
  logger_mt.cpp
  memory.hpp
  memory.cpp
  numbers.hpp
  numbers.cpp
  observer_set.hpp
  optional_ptr.hpp
  processing_queue.hpp
  rep_weights.hpp
  rep_weights.cpp
  rpc_handler_interface.hpp
  rpcconfig.hpp
  rpcconfig.cpp
  rsnanoutils.hpp
  rsnanoutils.cpp
  signal_manager.hpp
  signal_manager.cpp
  stats.hpp
  stats.cpp
  stream.hpp
  stream.cpp
  threading.hpp
  threading.cpp
  timer.hpp
  timer.cpp
  tlsconfig.hpp
  tlsconfig.cpp
  tomlconfig.hpp
  tomlconfig.cpp
  utility.hpp
  utility.cpp
  walletconfig.hpp
  walletconfig.cpp
  work.hpp
  work.cpp)

target_link_libraries(
  nano_lib
  crypto_lib
  secure
  ipc_flatbuffers_lib
  rsnano_ffi
  ${CMAKE_DL_LIBS}
  Boost::asio
  Boost::circular_buffer
  Boost::dll
  Boost::log_setup
  Boost::log
  Boost::multiprecision
  Boost::program_options
  Boost::stacktrace)

target_compile_definitions(
  nano_lib
  PRIVATE -DMAJOR_VERSION_STRING=${CPACK_PACKAGE_VERSION_MAJOR}
          -DMINOR_VERSION_STRING=${CPACK_PACKAGE_VERSION_MINOR}
          -DPATCH_VERSION_STRING=${CPACK_PACKAGE_VERSION_PATCH}
          -DPRE_RELEASE_VERSION_STRING=${CPACK_PACKAGE_VERSION_PRE_RELEASE}
          -DCI=${CI_TEST}
  PUBLIC -DACTIVE_NETWORK=${ACTIVE_NETWORK})
