cmake_minimum_required(VERSION 3.7)

project(shanghai)

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

# Main executable
file(GLOB_RECURSE SRCS "src/*.cpp")
add_executable(${PROJECT_NAME} ${SRCS})

# Scripts
configure_file(
	"script/cron.txt.in"
	"${CMAKE_CURRENT_BINARY_DIR}/cron.txt"
	@ONLY)

# Test executable
file(GLOB_RECURSE TEST_SRCS "test/*.cpp")
add_executable(${PROJECT_NAME}-test ${SRCS} ${TEST_SRCS})
target_compile_definitions(${PROJECT_NAME}-test PRIVATE "DISABLE_MAIN")

# Build info
find_package(Git REQUIRED)
set(BUILD_INFO_FILE "buildinfo_cmake.h")
set(BUILD_INFO_TMP "buildinfo_cmake.h.tmp")
if (CMAKE_BUILD_TYPE)
	set(BUILD_TYPE_DEF ${CMAKE_BUILD_TYPE})
else()
	set(BUILD_TYPE_DEF "Unknown")
endif()
add_custom_command(OUTPUT "__always_generate"
	COMMAND ${CMAKE_COMMAND} "-E" "remove" "-f" "${BUILD_INFO_TMP}"
	COMMAND ${CMAKE_COMMAND} "-E" "touch" "${BUILD_INFO_TMP}"
	COMMAND ${CMAKE_COMMAND} "-E" "echo"
		"\\#define" "BUILD_INFO_BUILD_TYPE" "\\\"${BUILD_TYPE_DEF}\\\""
		">>" "${BUILD_INFO_TMP}"
	COMMAND ${CMAKE_COMMAND} "-E" "echo"
		"\\#define" "BUILD_INFO_GIT_BRANCH"
		"\\\"`" "${GIT_EXECUTABLE}" "rev-parse" "--symbolic-full-name" "HEAD" "`\\\""
		">>" "${BUILD_INFO_TMP}"
	COMMAND ${CMAKE_COMMAND} "-E" "echo"
		"\\#define" "BUILD_INFO_GIT_HASH"
		"\\\"`" "${GIT_EXECUTABLE}" "rev-parse" "HEAD" "`\\\""
		">>" "${BUILD_INFO_TMP}"

	COMMAND ${CMAKE_COMMAND} "-E" "compare_files" ${BUILD_INFO_FILE} ${BUILD_INFO_TMP} ";"
		"if" "[" "$$?" "-eq" "0" "];" "then"
			${CMAKE_COMMAND} "-E" "echo" "No difference" ${BUILD_INFO_TMP} ${BUILD_INFO_FILE} ";"
		"else"
			${CMAKE_COMMAND} "-E" "echo" "copy" ${BUILD_INFO_TMP} ${BUILD_INFO_FILE} ";"
			${CMAKE_COMMAND} "-E" "copy" ${BUILD_INFO_TMP} ${BUILD_INFO_FILE} ";"
		"fi"

	COMMENT "Generating ${BUILD_INFO_FILE}..."
	)
add_custom_target(buildinfo DEPENDS "__always_generate")
add_dependencies(${PROJECT_NAME} buildinfo)
add_dependencies(${PROJECT_NAME}-test buildinfo)
target_include_directories(${PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_BINARY_DIR})
target_include_directories(${PROJECT_NAME}-test PRIVATE ${CMAKE_CURRENT_BINARY_DIR})

# Dependencies
# std::thread depends on thread
set(INCS ${CURL_INCLUDE_DIRS} ${MHD_INCLUDE_DIRS})
set(LIBS
	"Threads::Threads"
	"OpenSSL::SSL" ${CURL_LIBRARIES} ${MHD_LIBRARIES})
target_include_directories(${PROJECT_NAME} PRIVATE ${INCS})
target_include_directories(${PROJECT_NAME}-test PRIVATE ${INCS})
target_link_libraries(${PROJECT_NAME} ${LIBS})
target_link_libraries(${PROJECT_NAME}-test ${LIBS})

# submodules
target_link_libraries(${PROJECT_NAME} "json11")
target_link_libraries(${PROJECT_NAME}-test "json11")
target_link_libraries(${PROJECT_NAME}-test "gtest_main")

# install
install(TARGETS ${PROJECT_NAME}
	RUNTIME DESTINATION ".")
install(DIRECTORY "res/" DESTINATION "."
	FILE_PERMISSIONS OWNER_READ OWNER_WRITE
	DIRECTORY_PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/cron.txt" DESTINATION ".")

# make run
add_custom_target("run"
	COMMAND "./${PROJECT_NAME}"
	WORKING_DIRECTORY "${CMAKE_INSTALL_PREFIX}")
# make start
add_custom_target("start"
	COMMAND "./${PROJECT_NAME}" "--daemon"
	WORKING_DIRECTORY "${CMAKE_INSTALL_PREFIX}")
# make stop
add_custom_target("stop"
	COMMAND "xargs" "-t" "-a" "${PROJECT_NAME}.pid" "kill"
	WORKING_DIRECTORY "${CMAKE_INSTALL_PREFIX}")
# make reload
add_custom_target("reload"
	COMMAND "xargs" "-t" "-a" "${PROJECT_NAME}.pid" "kill" "-SIGHUP"
	WORKING_DIRECTORY "${CMAKE_INSTALL_PREFIX}")
# make flush
add_custom_target("flush"
	COMMAND "xargs" "-t" "-a" "${PROJECT_NAME}.pid" "kill" "-SIGUSR1"
	WORKING_DIRECTORY "${CMAKE_INSTALL_PREFIX}")

# make fulltest
add_custom_target("fulltest"
COMMAND $<TARGET_FILE:${PROJECT_NAME}-test>)
# make shorttest
add_custom_target("shorttest"
	COMMAND $<TARGET_FILE:${PROJECT_NAME}-test> "--gtest_filter=-*_SLOW")
