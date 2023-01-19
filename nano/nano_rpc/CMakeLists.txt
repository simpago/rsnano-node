add_executable(nano_rpc entry.cpp)

target_link_libraries(
  nano_rpc
  node
  rpc
  secure
  Boost::filesystem
  Boost::log_setup
  Boost::log
  Boost::program_options
  Boost::system
  Boost::thread)

target_compile_definitions(
  nano_rpc
  PUBLIC -DACTIVE_NETWORK=${ACTIVE_NETWORK}
  PRIVATE -DTAG_VERSION_STRING=${TAG_VERSION_STRING}
          -DGIT_COMMIT_HASH=${GIT_COMMIT_HASH})
