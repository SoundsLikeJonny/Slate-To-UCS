# The REAPER extension. Built out of tree from the Cargo workspace, because it links
# against REAPER's SDK rather than Rust.
#
# The SDK headers are not vendored. Run `cargo xtask fetch-reaper-sdk` first; the
# configure step fails with that instruction rather than a compiler error.

cmake_minimum_required(VERSION 3.20)
project(slate_reaper LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(SLATE_SDK_DIR "${CMAKE_CURRENT_SOURCE_DIR}/vendor" CACHE PATH
    "Directory holding reaper_plugin.h and reaper_plugin_functions.h")

if(NOT EXISTS "${SLATE_SDK_DIR}/reaper_plugin.h")
  message(FATAL_ERROR
    "REAPER SDK headers not found in ${SLATE_SDK_DIR}.\n"
    "Run `cargo xtask fetch-reaper-sdk` from the repository root first.")
endif()

set(SLATE_SOURCES
  src/launcher.cpp
  src/marshal.cpp
  src/notes.cpp
  src/queue.cpp
  src/slate_reaper.cpp
  src/undo.cpp
)

add_library(slate_reaper SHARED ${SLATE_SOURCES})

target_include_directories(slate_reaper PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/src"
)

# The SDK is Cockos's code, not ours: its unused-parameter warnings must not fail our
# build, so it is included as a system directory rather than with -Werror applied.
target_include_directories(slate_reaper SYSTEM PRIVATE "${SLATE_SDK_DIR}")

# Warnings are errors, matching the Rust side's `-D warnings`.
if(MSVC)
  target_compile_options(slate_reaper PRIVATE /W4 /WX /permissive-)
  # getenv is flagged as unsafe by MSVC's CRT deprecation list. The alternative,
  # _dupenv_s, allocates and needs a matching free at every call site; the reads here are
  # of process-owned environment strings that outlive the call.
  target_compile_definitions(slate_reaper PRIVATE _CRT_SECURE_NO_WARNINGS)
  target_link_libraries(slate_reaper PRIVATE ws2_32)
  set_target_properties(slate_reaper PROPERTIES OUTPUT_NAME "slate_reaper")
else()
  target_compile_options(slate_reaper PRIVATE -Wall -Wextra -Werror -Wpedantic)
  set_target_properties(slate_reaper PROPERTIES OUTPUT_NAME "slate_reaper")
endif()

# REAPER loads the extension by name, so the artefact must not carry a `lib` prefix on
# Unix and must be a `.dylib` on macOS rather than a versioned `.so`.
set_target_properties(slate_reaper PROPERTIES PREFIX "")

install(TARGETS slate_reaper
  LIBRARY DESTINATION .
  RUNTIME DESTINATION .
)

# The pure-logic tests need no REAPER and no SDK beyond the header types, so they run on
# every platform in CI.
option(SLATE_BUILD_TESTS "Build the extension's unit tests" ON)
if(SLATE_BUILD_TESTS)
  enable_testing()
  add_executable(slate_reaper_tests
    tests/test_main.cpp
    src/marshal.cpp
    src/notes.cpp
    src/queue.cpp
  )
  target_include_directories(slate_reaper_tests PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/src"
  )
  target_include_directories(slate_reaper_tests SYSTEM PRIVATE "${SLATE_SDK_DIR}")
  if(MSVC)
    target_compile_options(slate_reaper_tests PRIVATE /W4 /WX /permissive-)
    target_compile_definitions(slate_reaper_tests PRIVATE _CRT_SECURE_NO_WARNINGS)
  else()
    target_compile_options(slate_reaper_tests PRIVATE -Wall -Wextra -Werror -Wpedantic)
  endif()
  add_test(NAME slate_reaper_tests COMMAND slate_reaper_tests)
endif()
