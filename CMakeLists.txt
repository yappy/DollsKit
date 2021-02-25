cmake_minimum_required(VERSION 3.7)

project(DollsKit
	DESCRIPTION "House management system"
	HOMEPAGE_URL "https://github.com/yappy/DollsKit")

# Opus library requires NEON fpu option
if (${CMAKE_SYSTEM_PROCESSOR} MATCHES "^arm")
	set(DOLL_ARM_COMPILE_OPTIONS -mcpu=native -mfpu=auto
		CACHE STRING
		"ARM specific compile options (Edit manually if cross compile)")
	add_compile_options(${DOLL_ARM_COMPILE_OPTIONS})
endif()

# Set install dir
set(CMAKE_INSTALL_PREFIX "${CMAKE_CURRENT_BINARY_DIR}/dist")

#
# Libraries
#

find_package(PkgConfig REQUIRED)

# thread library for std::thread
set(THREADS_PREFER_PTHREAD_FLAG ON)
find_package(Threads REQUIRED)

# OpenSSL
find_package(OpenSSL REQUIRED)

# libcurl
find_package(CURL REQUIRED)

# libmicrohttpd
pkg_check_modules(MHD libmicrohttpd REQUIRED)

#
# submodules
#
add_subdirectory("external/googletest")
add_subdirectory("external/json11")
add_subdirectory("external/sleepy-discord")
# override sleepy-discord build option
set(ENABLE_VOICE ON CACHE BOOL "Enable voice support (override)" FORCE)

#
# projects
#
add_subdirectory("shanghai")
add_subdirectory("document")
