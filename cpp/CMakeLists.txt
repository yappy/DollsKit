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

add_subdirectory("shanghai")
