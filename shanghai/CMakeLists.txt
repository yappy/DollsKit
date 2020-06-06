cmake_minimum_required(VERSION 3.7)

project(shanghai)

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
set(BUILD_INFO_FILE "buildinfo.h")
find_package(Git REQUIRED)
add_custom_command(OUTPUT ${BUILD_INFO_FILE}
	COMMAND "${CMAKE_COMMAND}" "-E" "echo"
		"\\#define" "GIT_BRANCH"
		"\\\"`" "${GIT_EXECUTABLE}" "rev-parse" "--symbolic-full-name" "HEAD" "`\\\""
		">" "${BUILD_INFO_FILE}"
	COMMAND "${CMAKE_COMMAND}" "-E" "echo"
		"\\#define" "GIT_HASH"
		"\\\"`" "${GIT_EXECUTABLE}" "rev-parse" "HEAD" "`\\\""
		">>" "${BUILD_INFO_FILE}"
	)
add_custom_target(buildinfo DEPENDS ${BUILD_INFO_FILE})
add_dependencies(${PROJECT_NAME} buildinfo)
target_include_directories(${PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_BINARY_DIR})

# Dependencies
# std::thread depends on thread
set(INCS ${CURL_INCLUDE_DIRS} ${MHD_INCLUDE_DIRS})
set(LIBS
	"Threads::Threads"
	"OpenSSL::SSL" ${CURL_LIBRARIES} ${MHD_LIBRARIES} "json11")
target_include_directories(${PROJECT_NAME} PRIVATE ${INCS})
target_include_directories(${PROJECT_NAME}-test PRIVATE ${INCS})
target_link_libraries(${PROJECT_NAME} ${LIBS})
target_link_libraries(${PROJECT_NAME}-test "gtest_main" "gtest" ${LIBS})

# External projects must be built before this
add_dependencies(${PROJECT_NAME} JSON11)
add_dependencies(${PROJECT_NAME}-test GTEST JSON11)

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

# make fulltest
add_custom_target("fulltest"
COMMAND $<TARGET_FILE:${PROJECT_NAME}-test>)
# make shorttest
add_custom_target("shorttest"
	COMMAND $<TARGET_FILE:${PROJECT_NAME}-test> "--gtest_filter=-*_SLOW")
