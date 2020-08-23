cmake_minimum_required(VERSION 3.7)

project(DollsKit
	DESCRIPTION "House management system"
	HOMEPAGE_URL "https://github.com/yappy/DollsKit")

# Set install dir
set(CMAKE_INSTALL_PREFIX "${CMAKE_CURRENT_BINARY_DIR}/dist")

# -std=c++17
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# warning options
if(${CMAKE_CXX_COMPILER_ID} MATCHES "Clang")
	add_compile_options(-Wall -Wextra
		-Wno-c++98-compat -Wno-c++98-compat-pedantic
		-Wno-unused-parameter -Wno-unused-variable)
elseif(${CMAKE_CXX_COMPILER_ID} STREQUAL "GNU")
	add_compile_options(-Werror -Wall -Wextra -Wnoexcept -Wno-unused)
elseif(${CMAKE_CXX_COMPILER_ID} STREQUAL "MSVC")
	add_compile_options(/W4)
endif()

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

#
# projects
#
add_subdirectory("shanghai")
add_subdirectory("document")
