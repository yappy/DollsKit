cmake_minimum_required(VERSION 3.7)

project(DollsKit
	DESCRIPTION "House management system"
	HOMEPAGE_URL "https://github.com/yappy/DollsKit")

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

#
# projects
#
add_subdirectory("shanghai")
add_subdirectory("document")
