cmake_minimum_required(VERSION 3.7)

project(shanghai)

file(GLOB_RECURSE SRCS "src/*.cpp")
add_executable(${PROJECT_NAME} ${SRCS})

file(GLOB_RECURSE TEST_SRCS "test/*.cpp")
add_executable(${PROJECT_NAME}-test ${SRCS} ${TEST_SRCS})
target_compile_definitions(${PROJECT_NAME}-test PRIVATE "DISABLE_MAIN")
add_test(
	NAME "${PROJECT_NAME}-test"
	COMMAND "$<TARGET_FILE:${PROJECT_NAME}-test>")

# std::thread depends on thread
set(INCS ${CURL_INCLUDE_DIRS})
set(LIBS "Threads::Threads" ${CURL_LIBRARIES} "json11")
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
install(DIRECTORY "res/" DESTINATION ".")

# make run
add_custom_target("run"
	COMMAND "./${PROJECT_NAME}"
	WORKING_DIRECTORY "${CMAKE_INSTALL_PREFIX}")
