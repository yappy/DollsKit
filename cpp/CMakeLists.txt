cmake_minimum_required(VERSION 3.7)

# Set install dir
set(CMAKE_INSTALL_PREFIX "${CMAKE_CURRENT_BINARY_DIR}/dist")

# -std=c++14
set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# warning options
if(${CMAKE_CXX_COMPILER_ID} MATCHES "Clang")
	add_compile_options(-Wall -Weverything -Wno-c++98-compat -Wno-c++98-compat-pedantic)
elseif(${CMAKE_CXX_COMPILER_ID} STREQUAL "GNU")
	add_compile_options(-Werror -Wall -Wextra -Wnoexcept -Wno-unused)
elseif(${CMAKE_CXX_COMPILER_ID} STREQUAL "MSVC")
	add_compile_options(/W4)
endif()

#
# Libraries
#

# thread library for std::thread
set(THREADS_PREFER_PTHREAD_FLAG ON)
find_package(Threads REQUIRED)

# OpenSSL
find_package(OpenSSL REQUIRED)

# libcurl
find_package(CURL REQUIRED)


#
# Externals
#
include(ExternalProject)
include_directories(
	"${CMAKE_CURRENT_BINARY_DIR}/include"
	"${CMAKE_CURRENT_BINARY_DIR}/include/${CMAKE_LIBRARY_ARCHITECTURE}")
link_directories(
	"${CMAKE_CURRENT_BINARY_DIR}/lib"
	"${CMAKE_CURRENT_BINARY_DIR}/lib/${CMAKE_LIBRARY_ARCHITECTURE}")

# gtest
ExternalProject_Add(GTEST
	PREFIX "${CMAKE_CURRENT_BINARY_DIR}/external/googletest"
	INSTALL_DIR "${CMAKE_CURRENT_BINARY_DIR}"
	# download
	GIT_REPOSITORY "https://github.com/google/googletest"
	GIT_TAG "release-1.8.0"
	GIT_SHALLOW 1
	# update
	# update causes build slow down...
	# version is fixed to be used; no update
	UPDATE_COMMAND ""
	# configure
	CMAKE_ARGS
		"-DCMAKE_INSTALL_PREFIX=${CMAKE_CURRENT_BINARY_DIR}"
		"-DBUILD_GMOCK=OFF"
		"-DBUILD_GTEST=ON"
	# build
	# install
)

# json11
ExternalProject_Add(JSON11
	PREFIX "${CMAKE_CURRENT_BINARY_DIR}/external/json11"
	INSTALL_DIR "${CMAKE_CURRENT_BINARY_DIR}"
	# download
	GIT_REPOSITORY "https://github.com/dropbox/json11"
	# 21 Jun 2017
	GIT_TAG "ec4e45219af1d7cde3d58b49ed762376fccf1ace"
	GIT_SHALLOW 1
	# update
	# update causes build slow down...
	# version is fixed to be used; no update
	UPDATE_COMMAND ""
	# configure
	CMAKE_ARGS "-DCMAKE_INSTALL_PREFIX=${CMAKE_CURRENT_BINARY_DIR}"
	# build
	# install
)

#
# projects
#
add_subdirectory("shanghai")
