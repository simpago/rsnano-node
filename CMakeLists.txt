cmake_minimum_required(VERSION 3.14)

if(CMAKE_VERSION VERSION_GREATER 3.12 OR CMAKE_VERSION VERSION_EQUAL 3.12)
  # find_package uses <PACKAGENAME>_ROOT variables
  cmake_policy(SET CMP0074 NEW)
endif()
if(CMAKE_VERSION VERSION_GREATER 3.13 OR CMAKE_VERSION VERSION_EQUAL 3.13)
  # option honors normal variables
  cmake_policy(SET CMP0077 NEW)
endif()

# OSX compatibility needs to be set before project is declared
set(CMAKE_OSX_DEPLOYMENT_TARGET
    13.3
    CACHE STRING "")

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
project(nano-node)

# Build the Rust part
# ---------------------
find_package(Corrosion REQUIRED)
corrosion_import_crate(MANIFEST_PATH rust/ffi/Cargo.toml)
# ---------------------

find_package(OpenSSL)

# Get the latest abbreviated commit hash of the working branch
execute_process(
  COMMAND git log -1 --format=%h
  WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
  OUTPUT_VARIABLE GIT_COMMIT_HASH
  OUTPUT_STRIP_TRAILING_WHITESPACE)

option(CI_VERSION_PRE_RELEASE false)

set(COVERAGE
    OFF
    CACHE BOOL "Code Coverage Targets")
if(COVERAGE)
  list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}/cmake/Modules")
  include(CoverageTest)
endif()

if(MSVC)
  add_definitions(/MP)
endif()

set(CPACK_PACKAGE_VENDOR "RsNano")
set(CPACK_PACKAGE_VERSION_MAJOR "2")
set(CPACK_PACKAGE_VERSION_MINOR "0")
set(CPACK_PACKAGE_VERSION_PATCH "0")

if(CI_VERSION_PRE_RELEASE)
  set(CPACK_PACKAGE_VERSION_PRE_RELEASE "${CI_VERSION_PRE_RELEASE}")
else()
  set(CPACK_PACKAGE_VERSION_PRE_RELEASE "4")
endif()

if(CI_TAG)
  set(TAG_VERSION_STRING "${CI_TAG}")
else()
  set(TAG_VERSION_STRING
      "V${CPACK_PACKAGE_VERSION_MAJOR}.${CPACK_PACKAGE_VERSION_MINOR}")
endif()

if(APPLE)
  set(CMAKE_INSTALL_RPATH "@executable_path/../Frameworks")
else()
  set(CMAKE_INSTALL_RPATH "$ORIGIN/../lib")
endif()

# Create all libraries and executables in the root binary dir
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_DEBUG ${CMAKE_BINARY_DIR})
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_RELEASE ${CMAKE_BINARY_DIR})

set(NANO_TEST
    OFF
    CACHE BOOL "")
set(NANO_WARN_TO_ERR
    OFF
    CACHE BOOL "")
set(NANO_TIMED_LOCKS
    0
    CACHE STRING "")
set(NANO_TIMED_LOCKS_IGNORE_BLOCKED
    OFF
    CACHE BOOL "")
set(NANO_TIMED_LOCKS_FILTER
    ""
    CACHE
      STRING
      "Selects which mutex should be the only one to have stacktraces generated, empty string means report on all"
)
set_property(
  CACHE NANO_TIMED_LOCKS_FILTER
  PROPERTY STRINGS
           active
           block_arrival
           block_processor
           dropped_elections,
           election_winner_details
           gap_cache
           network_filter
           observer_set
           request_aggregator
           state_block_signature_verification
           telemetry
           vote_generator
           vote_processor
           votes_cache
           work_pool)
set(NANO_ASIO_HANDLER_TRACKING
    0
    CACHE STRING "")

option(NANO_STACKTRACE_BACKTRACE
       "Use BOOST_STACKTRACE_USE_BACKTRACE in stacktraces, for POSIX" OFF)

if(NANO_STACKTRACE_BACKTRACE)
  add_definitions(-DNANO_STACKTRACE_BACKTRACE)
  add_definitions(-DBOOST_STACKTRACE_USE_BACKTRACE)
  if(NANO_BACKTRACE_INCLUDE)
    add_definitions(
      -DBOOST_STACKTRACE_BACKTRACE_INCLUDE_FILE=${NANO_BACKTRACE_INCLUDE})
  endif()
endif()

# Enable NANO_TRACING by default in Debug builds
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
  option(NANO_TRACING "Enable trace logging" ON)
else()
  option(NANO_TRACING "Enable trace logging" OFF)
endif()
if(NANO_TRACING)
  message(STATUS "Using trace logging")
  add_compile_definitions(NANO_TRACING)
endif()

if(${NANO_TIMED_LOCKS} GREATER 0)
  add_definitions(-DNANO_TIMED_LOCKS=${NANO_TIMED_LOCKS})
  add_definitions(-DNANO_TIMED_LOCKS_FILTER=${NANO_TIMED_LOCKS_FILTER})
  if(NANO_TIMED_LOCKS_IGNORE_BLOCKED)
    add_definitions(-DNANO_TIMED_LOCKS_IGNORE_BLOCKED)
  endif()
endif()

if(${NANO_ASIO_HANDLER_TRACKING} GREATER 0)
  add_definitions(-DNANO_ASIO_HANDLER_TRACKING=${NANO_ASIO_HANDLER_TRACKING}
                  -DBOOST_ASIO_ENABLE_HANDLER_TRACKING)
endif()

option(NANO_SIMD_OPTIMIZATIONS
       "Enable CPU-specific SIMD optimizations (SSE/AVX or NEON, e.g.)" OFF)
option(
  ENABLE_AES
  "Enable AES optimizations (enabled by default with NANO_SIMD_OPTIMIZATIONS, set OFF to disable"
  ON)
option(ENABLE_AVX2 "Enable AVX2 optimizations" OFF)

set(ACTIVE_NETWORK
    nano_live_network
    CACHE STRING "Selects which network parameters are used")
set_property(
  CACHE ACTIVE_NETWORK PROPERTY STRINGS nano_dev_network nano_beta_network
                                nano_live_network nano_test_network)
set(CPACK_PACKAGE_NAME
    "nano-node"
    CACHE STRING "" FORCE)
set(CPACK_NSIS_PACKAGE_NAME
    "Nano"
    CACHE STRING "" FORCE)
set(CPACK_PACKAGE_INSTALL_DIRECTORY
    "nanocurrency"
    CACHE STRING "" FORCE)
set(NANO_SERVICE "nanocurrency.service")
set(NANO_PREFIX "")
if("${ACTIVE_NETWORK}" MATCHES "nano_beta_network")
  project("nano-node-beta")
  set(CPACK_PACKAGE_NAME
      "nano-node-beta"
      CACHE STRING "" FORCE)
  set(CPACK_NSIS_PACKAGE_NAME
      "Nano-Beta"
      CACHE STRING "" FORCE)
  set(CPACK_PACKAGE_INSTALL_DIRECTORY
      "nanocurrency-beta"
      CACHE STRING "" FORCE)
  set(NANO_SERVICE "nanocurrency-beta.service")
  set(NANO_PREFIX "Beta")
elseif("${ACTIVE_NETWORK}" MATCHES "nano_test_network")
  project("nano-node-test")
  set(CPACK_PACKAGE_NAME
      "nano-node-test"
      CACHE STRING "" FORCE)
  set(CPACK_NSIS_PACKAGE_NAME
      "Nano-Test"
      CACHE STRING "" FORCE)
  set(CPACK_PACKAGE_INSTALL_DIRECTORY
      "nanocurrency-test"
      CACHE STRING "" FORCE)
  set(NANO_SERVICE "nanocurrency-test.service")
  set(NANO_PREFIX "Test")
endif()
set(NANO_OSX_PACKAGE_NAME
    ${CPACK_NSIS_PACKAGE_NAME}
    CACHE STRING "" FORCE)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE
      "Release"
      CACHE
        STRING
        "Choose the type of build, options are: Debug Release RelWithDebInfo MinSizeRel."
        FORCE)
endif()

set(CMAKE_POSITION_INDEPENDENT_CODE ON)

find_package(Threads)

# Sanitizers
option(NANO_ASAN "Enable ASan" OFF)
if(NANO_ASAN)
  if(MSVC)
    message(FATAL_ERROR "ASan is not supported on MSVC")
  endif()

  message(STATUS "Using Asan")
  add_compile_options(-fsanitize=address)
  add_link_options(-fsanitize=address)

  if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    add_compile_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_asan")
    add_link_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_asan")
  endif()

  add_definitions(-DED25519_NO_INLINE_ASM)
endif()

option(NANO_ASAN_INT "Enable ASan+Integer overflow" OFF)
if(NANO_ASAN_INT)
  if(MSVC)
    message(FATAL_ERROR "ASan+Integer overflow is not supported on MSVC")
  endif()

  message(STATUS "Using ASan+Integer overflow")
  add_compile_options(-fsanitize=address,integer)
  add_link_options(-fsanitize=address,integer)

  if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    add_compile_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_asan")
    add_link_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_asan")
  endif()

  add_definitions(-DED25519_NO_INLINE_ASM)
endif()

option(NANO_UBSAN "Enable UBSan" OFF)
if(NANO_UBSAN)
  if(MSVC)
    message(FATAL_ERROR "UBSan is not supported on MSVC")
  endif()

  message(STATUS "Using UBSan")
  add_compile_options(-fsanitize=undefined)
  add_link_options(-fsanitize=undefined)

  if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    add_compile_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_ubsan")
    add_link_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_ubsan")
  endif()

  add_definitions(-DED25519_NO_INLINE_ASM)
  add_definitions(-DROCKSDB_UBSAN_RUN)
endif()

option(NANO_TSAN "Enable TSan" OFF)
if(NANO_TSAN)
  if(MSVC)
    message(FATAL_ERROR "TSan is not supported on MSVC")
  endif()

  message(STATUS "Using TSan")
  add_compile_options(-fsanitize=thread)
  add_link_options(-fsanitize=thread)

  if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    add_compile_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_tsan")
    add_link_options(
      "-fsanitize-ignorelist=${PROJECT_SOURCE_DIR}/sanitize_ignorelist_tsan")
  endif()

  add_definitions(-DED25519_NO_INLINE_ASM)
endif()

if(NANO_WARN_TO_ERR)
  add_compile_options(-Werror -Wno-deprecated-declarations)
endif()

if(WIN32)
  find_library(PSAPI Psapi)
  add_definitions(-D_WIN32_WINNT=0x0600 -DWINVER=0x0600
                  -D_CRT_SECURE_NO_WARNINGS -DNOGDI /EHsc)
else()
  if(NANO_FUZZER_TEST)
    add_compile_options(-fsanitize=fuzzer-no-link -fno-omit-frame-pointer)
    add_definitions(-DNANO_FUZZER_TEST)
  endif()

  if(CMAKE_SYSTEM_PROCESSOR MATCHES "^(i.86|x86(_64)?)$")
    if(NANO_SIMD_OPTIMIZATIONS
       OR RAIBLOCKS_SIMD_OPTIMIZATIONS
       OR ENABLE_AVX2)
      add_compile_options(-msse4)
      if(ENABLE_AES)
        add_compile_options(-maes)
      endif()
    elseif(CMAKE_SYSTEM_PROCESSOR STREQUAL "x86_64")
      add_compile_options(-msse2)
    endif()

  elseif(CMAKE_SYSTEM_PROCESSOR STREQUAL "aarch64")
    if(NANO_SIMD_OPTIMIZATIONS OR RAIBLOCKS_SIMD_OPTIMIZATIONS)
      add_compile_options(-march=armv8-a+crc+crypto)
    else()
      add_compile_options(-march=armv8-a)
    endif()
  endif()
endif()

if(ENABLE_AVX2)
  if(WIN32)
    add_definitions(/arch:AVX2)
  elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "^(i.86|x86(_64)?)$")
    add_compile_options(-mavx2 -mbmi -mbmi2 -maes)
  endif()
  if(PERMUTE_WITH_GATHER)
    add_definitions(-DPERMUTE_WITH_GATHER)
  elseif(PERMUTE_WITH_SHUFFLES)
    add_definitions(-DPERMUTE_WITH_SHUFFLES)
  endif()
endif()

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
# set(CMAKE_C_EXTENSIONS OFF)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
# set(CMAKE_CXX_EXTENSIONS OFF)

if(WIN32)
  set(PLATFORM_LINK_FLAGS "")
  add_definitions(/bigobj)
else()
  if(APPLE)
    set(PLATFORM_LINK_FLAGS "-framework Foundation -framework OpenCL")
  else()
    set(PLATFORM_LINK_FLAGS "-static-libgcc -static-libstdc++")
  endif()

  if(NANO_FUZZER_TEST)
    set(PLATFORM_LINK_FLAGS "${PLATFORM_LINK_FLAGS} -fsanitize=fuzzer-no-link")
  endif()
endif()

set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} ${PLATFORM_LINK_FLAGS}")

set(OPENSSL_LIBRARIES "")

include_directories(${CMAKE_SOURCE_DIR})
list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}/cmake/Modules")

set(Boost_INCLUDE_DIR ${CMAKE_SOURCE_DIR}/submodules/boost/libs/config/include)
set(BOOST_MODULE_LIBS
    algorithm
    align
    any
    array
    asio
    assert
    atomic
    beast
    bind
    chrono
    circular_buffer
    concept_check
    config
    container
    container_hash
    context
    conversion
    core
    coroutine
    date_time
    describe
    detail
    dll
    dynamic_bitset
    endian
    exception
    filesystem
    foreach
    format
    function
    function_types
    functional
    fusion
    integer
    interprocess
    intrusive
    io
    iostreams
    iterator
    lexical_cast
    property_tree
    lambda
    log
    logic
    math
    move
    mp11
    mpl
    multi_index
    multiprecision
    numeric/conversion
    optional
    parameter
    phoenix
    pool
    predef
    preprocessor
    process
    program_options
    proto
    random
    range
    ratio
    rational
    regex
    serialization
    smart_ptr
    spirit
    stacktrace
    static_assert
    static_string
    system
    test
    thread
    throw_exception
    tokenizer
    tuple
    type_index
    type_traits
    typeof
    unordered
    utility
    variant
    variant2
    winapi)

add_definitions(-DBOOST_ALL_NO_LIB) # Disable automatic boost linking
foreach(lib IN LISTS BOOST_MODULE_LIBS)
  add_subdirectory(submodules/boost/libs/${lib} EXCLUDE_FROM_ALL)
endforeach()
include_directories(${BOOST_LIBRARY_INCLUDES})
add_library(Boost::stacktrace ALIAS boost_stacktrace_basic)
add_definitions(-DBOOST_STACKTRACE_GNU_SOURCE_NOT_REQUIRED)

cmake_policy(SET CMP0079 NEW)
# Workaround for missing reference errata in the boost property_tree module
target_link_libraries(boost_property_tree INTERFACE Boost::any)
target_link_libraries(boost_property_tree INTERFACE Boost::format)
target_link_libraries(boost_property_tree INTERFACE Boost::multi_index)

# cpptoml
include_directories(submodules/cpptoml/include)

add_subdirectory(nano/ipc_flatbuffers_lib)
add_subdirectory(nano/ipc_flatbuffers_test)

# fmt
add_subdirectory(submodules/fmt EXCLUDE_FROM_ALL)
include_directories(submodules/fmt/include)

# spdlog
add_definitions(-DSPDLOG_FMT_EXTERNAL)
add_subdirectory(submodules/spdlog EXCLUDE_FROM_ALL)
include_directories(submodules/spdlog/include)

set(BUILD_SHARED
    OFF
    CACHE BOOL "")
set(BUILD_TESTING
    OFF
    CACHE BOOL "")
set(USE_INTERMEDIATE_OBJECTS_TARGET
    OFF
    CACHE BOOL "")

if(WIN32)
  target_link_libraries(ntdll)
endif()

add_subdirectory(nano/crypto_lib)
add_subdirectory(nano/secure)
add_subdirectory(nano/store)
target_include_directories(nano_store PUBLIC ${BOOST_LIBRARY_INCLUDES})
target_include_directories(nano_store PUBLIC submodules)
add_subdirectory(nano/lib)
add_subdirectory(nano/node)
add_subdirectory(nano/nano_node)
add_subdirectory(nano/rpc)

add_custom_target(
  executables
  COMMAND echo "BATCH BUILDING node + rpc"
  WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
  DEPENDS nano_node)

if(NANO_TEST OR RAIBLOCKS_TEST)
  if(WIN32)
    if(MSVC_VERSION)
      if(MSVC_VERSION GREATER_EQUAL 1910)
        add_definitions(-DGTEST_LANG_CXX11=1)
        add_definitions(-DGTEST_HAS_TR1_TUPLE=0)
      endif()
    endif()
    set(gtest_force_shared_crt ON)
  else()
    set(gtest_force_shared_crt OFF)
  endif()

  # FIXME: This fixes googletest GOOGLETEST_VERSION requirement
  set(GOOGLETEST_VERSION 1.11.0)
  add_subdirectory(submodules/gtest/googletest)
  # FIXME: This fixes gtest include directories without modifying gtest's
  # CMakeLists.txt. Ideally we should use GTest::GTest and GTest::Main as
  # dependencies but it requires building gtest differently
  set_target_properties(
    gtest PROPERTIES INTERFACE_INCLUDE_DIRECTORIES
                     "${CMAKE_SOURCE_DIR}/submodules/gtest/googletest/include")

  add_subdirectory(nano/test_common)
  add_subdirectory(nano/core_test)
  add_subdirectory(nano/rpc_test)
  add_custom_target(
    all_tests
    COMMAND echo "BATCH BUILDING TESTS"
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
    DEPENDS core_test load_test rpc_test nano_node)
endif()

if(NANO_TEST OR RAIBLOCKS_TEST)
  add_custom_target(
    quick_tests
    COMMAND echo "BATCH QUICK TESTS"
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
    DEPENDS core_test rpc_test)
endif()

if(COVERAGE)
  add_subdirectory(coverage)
endif()

set(CPACK_RESOURCE_FILE_LICENSE ${CMAKE_SOURCE_DIR}/LICENSE)
include(CPack)
