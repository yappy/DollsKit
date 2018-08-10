cmake_minimum_required(VERSION 3.7)

# Enable make datail output
set(CMAKE_VERBOSE_MAKEFILE 1)

# Set install dir
set(CMAKE_INSTALL_PREFIX "${CMAKE_CURRENT_BINARY_DIR}/dist")

# -std=c++14
set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(${CMAKE_CXX_COMPILER_ID} MATCHES "Clang")
	add_compile_options(-Wall -Weverything -Wno-c++98-compat -Wno-c++98-compat-pedantic)
elseif(${CMAKE_CXX_COMPILER_ID} STREQUAL "GNU")
	add_compile_options(-Werror -Wall -Wextra -Wnoexcept -Wno-unused)
elseif(${CMAKE_CXX_COMPILER_ID} STREQUAL "MSVC")
	add_compile_options(/W4)
endif()

#
# Externals
#
include_directories(
	"${CMAKE_CURRENT_BINARY_DIR}/include"
	"${CMAKE_CURRENT_BINARY_DIR}/${CMAKE_LIBRARY_ARCHITECTURE}/include")
link_directories(
	"${CMAKE_CURRENT_BINARY_DIR}/lib"
	"${CMAKE_CURRENT_BINARY_DIR}/${CMAKE_LIBRARY_ARCHITECTURE}/lib")

# json11
include(ExternalProject)
ExternalProject_Add(json11
	PREFIX "${CMAKE_CURRENT_BINARY_DIR}/external/json11"
	INSTALL_DIR "${CMAKE_CURRENT_BINARY_DIR}"
	# download
	GIT_REPOSITORY "https://github.com/dropbox/json11"
	# 21 Jun 2017
	GIT_TAG "ec4e45219af1d7cde3d58b49ed762376fccf1ace"
	# update
	# configure
	CMAKE_ARGS "-DCMAKE_INSTALL_PREFIX=${CMAKE_CURRENT_BINARY_DIR}"
	# build
	# install
)

# projects
add_subdirectory("shanghai")
