cmake_minimum_required(VERSION 3.4)

if(CMAKE_VERSION VERSION_GREATER 3.12 OR CMAKE_VERSION VERSION_EQUAL 3.12)
  # find_package uses <PACKAGENAME>_ROOT variables
  cmake_policy(SET CMP0074 NEW)
endif()
if(CMAKE_VERSION VERSION_GREATER 3.13 OR CMAKE_VERSION VERSION_EQUAL 3.13)
  # option honors normal variables
  cmake_policy(SET CMP0077 NEW)
endif()

# compatibility for osx sierra and on needs to be set before project
set(CMAKE_OSX_DEPLOYMENT_TARGET
    10.14
    CACHE STRING "")

project(nano-node)

# Build the Rust part
# ---------------------
find_package(Corrosion REQUIRED)
corrosion_import_crate(MANIFEST_PATH rust/ffi/Cargo.toml)
# ---------------------

# Get the latest abbreviated commit hash of the working branch
execute_process(
  COMMAND git log -1 --format=%h
  WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
  OUTPUT_VARIABLE GIT_COMMIT_HASH
  OUTPUT_STRIP_TRAILING_WHITESPACE)

option(CI_BUILD false)
option(CI_VERSION_PRE_RELEASE false)

set(COVERAGE
    OFF
    CACHE BOOL "Code Coverage Targets")
if(COVERAGE)
  list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}/cmake/Modules")
  include(CoverageTest)
endif()

set(CI_TEST
    0
    CACHE STRING "")

if(MSVC)
  add_definitions(/MP)
endif()

set(CPACK_PACKAGE_VERSION_MAJOR "25")
set(CPACK_PACKAGE_VERSION_MINOR "0")
set(CPACK_PACKAGE_VERSION_PATCH "0")
if(CI_BUILD AND CI_VERSION_PRE_RELEASE)
  set(CPACK_PACKAGE_VERSION_PRE_RELEASE "${CI_VERSION_PRE_RELEASE}")
else()
  set(CPACK_PACKAGE_VERSION_PRE_RELEASE "99")
endif()
set(CPACK_PACKAGE_VENDOR "Nano Currency")

if(CI_BUILD)
  set(TAG_VERSION_STRING "$ENV{CI_TAG}")
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

set(NANO_TEST
    OFF
    CACHE BOOL "")
set(NANO_SECURE_RPC
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
           block_uniquer
           confirmation_height_processor
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
           vote_uniquer
           votes_cache
           work_pool)
set(NANO_FUZZER_TEST
    OFF
    CACHE BOOL "")
set(NANO_ASIO_HANDLER_TRACKING
    0
    CACHE STRING "")

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

option(NANO_ASAN_INT "Enable ASan+UBSan+Integer overflow" OFF)
option(NANO_ASAN "Enable ASan+UBSan" OFF)
option(NANO_TSAN "Enable TSan" OFF)
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

set(USING_ASAN (NANO_ASAN OR RAIBLOCKS_ASAN))
set(USING_ASAN_INT (NANO_ASAN_INT OR RAIBLOCKS_ASAN_INT))
set(USING_TSAN (NANO_TSAN OR RAIBLOCKS_TSAN))

find_package(Threads)

if(WIN32)
  find_library(PSAPI Psapi)
  add_definitions(-D_WIN32_WINNT=0x0600 -DWINVER=0x0600 -DWIN32_LEAN_AND_MEAN
                  -DMINIUPNP_STATICLIB -D_CRT_SECURE_NO_WARNINGS /EHsc)

  if(${USING_TSAN}
     OR ${USING_ASAN}
     OR ${USING_ASAN_INT})
    message(WARNING "Cannot use TSAN or ASAN on Windows, sanitizers ignored")
  endif()

else()
  if(NANO_WARN_TO_ERR)
    add_compile_options(-Werror -Wno-deprecated-declarations)
  endif()

  if((${USING_TSAN} AND ${USING_ASAN}) OR (${USING_TSAN} AND ${USING_ASAN_INT}))
    message(WARNING "Cannot use TSAN/ASAN together, defaulting to ASAN")
  endif()

  if(${USING_ASAN} OR ${USING_ASAN_INT})
    if(${USING_ASAN_INT})
      add_compile_options(-fsanitize=address,undefined,integer)
    else()
      add_compile_options(-fsanitize=address,undefined)
    endif()
  elseif(${USING_TSAN})
    add_compile_options(-fsanitize=thread)
    if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
      add_compile_options(
        "-fsanitize-blacklist=${PROJECT_SOURCE_DIR}/tsan_clang_blacklist")
    endif()
  endif()

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

  if(${USING_ASAN_INT})
    set(PLATFORM_LINK_FLAGS
        "${PLATFORM_LINK_FLAGS} -fsanitize=address,undefined,integer")
  elseif(${USING_ASAN})
    set(PLATFORM_LINK_FLAGS
        "${PLATFORM_LINK_FLAGS} -fsanitize=address,undefined")
  elseif(${USING_TSAN})
    set(PLATFORM_LINK_FLAGS "${PLATFORM_LINK_FLAGS} -fsanitize=thread")
    if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
      set(PLATFORM_LINK_FLAGS
          "${PLATFORM_LINK_FLAGS} -fsanitize-blacklist=${PROJECT_SOURCE_DIR}/tsan_clang_blacklist"
      )
    endif()
  endif()
  if(NANO_FUZZER_TEST)
    set(PLATFORM_LINK_FLAGS "${PLATFORM_LINK_FLAGS} -fsanitize=fuzzer-no-link")
  endif()
endif()

set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} ${PLATFORM_LINK_FLAGS}")

if(NANO_SECURE_RPC OR RAIBLOCKS_SECURE_RPC)
  find_package(OpenSSL 1.0 REQUIRED)
  include_directories(${OPENSSL_INCLUDE_DIR})
  add_definitions(-DNANO_SECURE_RPC)
  message("OpenSSL include dir: ${OPENSSL_INCLUDE_DIR}")
  message("OpenSSL libraries: ${OPENSSL_LIBRARIES}")
  message("OpenSSL lib: ${OPENSSL_SSL_LIBRARY}")
  message("Crypto lib: ${OPENSSL_CRYPTO_LIBRARY}")
else()
  set(OPENSSL_LIBRARIES "")
endif()

include_directories(${CMAKE_SOURCE_DIR})
list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}/cmake/Modules")

set(Boost_INCLUDE_DIR ${CMAKE_SOURCE_DIR}/boost/libs/config/include)
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
  add_subdirectory(boost/libs/${lib} EXCLUDE_FROM_ALL)
endforeach()
include_directories(${BOOST_LIBRARY_INCLUDES})
add_library(Boost::stacktrace ALIAS boost_stacktrace_basic)
add_definitions(-DBOOST_STACKTRACE_GNU_SOURCE_NOT_REQUIRED)

# Workaround for GitHub builders which do not appear to have the Windows Message
# Compiler mc.exe
add_definitions(-DBOOST_LOG_WITHOUT_EVENT_LOG)

# Workaround for missing reference errata in the boost property_tree module
target_link_libraries(boost_property_tree INTERFACE Boost::any)
target_link_libraries(boost_property_tree INTERFACE Boost::format)
target_link_libraries(boost_property_tree INTERFACE Boost::multi_index)

# diskhash
if(NOT CMAKE_SYSTEM_NAME STREQUAL "Windows")
  add_library(diskhash STATIC ${CMAKE_SOURCE_DIR}/diskhash/src/diskhash.c)
  include_directories(diskhash/src)
endif()

include_directories(cpptoml/include)

add_subdirectory(nano/ipc_flatbuffers_lib)
add_subdirectory(nano/ipc_flatbuffers_test)

set(UPNPC_BUILD_SHARED
    OFF
    CACHE BOOL "")
add_subdirectory(miniupnp/miniupnpc EXCLUDE_FROM_ALL)

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
add_subdirectory(nano/lib)
add_subdirectory(nano/node)
add_subdirectory(nano/nano_node)
add_subdirectory(nano/rpc)
add_subdirectory(nano/nano_rpc)

if(NANO_FUZZER_TEST)
  if(NOT WIN32)
    add_subdirectory(nano/fuzzer_test)
  else()
    message(WARNING "Fuzzing is not supported on Windows")
  endif()
endif()

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
  add_subdirectory(gtest/googletest)
  # FIXME: This fixes gtest include directories without modifying gtest's
  # CMakeLists.txt. Ideally we should use GTest::GTest and GTest::Main as
  # dependencies but it requires building gtest differently
  set_target_properties(
    gtest PROPERTIES INTERFACE_INCLUDE_DIRECTORIES
                     "${CMAKE_SOURCE_DIR}/gtest/googletest/include")

  add_subdirectory(nano/test_common)
  add_subdirectory(nano/core_test)
  add_subdirectory(nano/rpc_test)
  add_subdirectory(nano/slow_test)
  add_custom_target(
    build_tests
    COMMAND echo "BATCH BUILDING TESTS"
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
    DEPENDS core_test rpc_test nano_node nano_rpc)
  add_custom_target(
    run_tests
    COMMAND ${PROJECT_SOURCE_DIR}/ci/test.sh ${CMAKE_BINARY_DIR}
    WORKING_DIRECTORY ${PROJECT_SOURCE_DIR}
    DEPENDS build_tests)
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
