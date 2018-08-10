cmake_minimum_required(VERSION 3.7)

project(shanghai)

file(GLOB SRCS "src/*.cpp")
add_executable(${PROJECT_NAME} ${SRCS})
file(GLOB TEST_SRCS "test/*.cpp")
add_executable(${PROJECT_NAME}-test ${SRCS} ${TEST_SRCS})
target_compile_definitions(${PROJECT_NAME}-test PRIVATE "DISABLE_MAIN")
add_test(
	NAME "${PROJECT_NAME}-test"
	COMMAND "$<TARGET_FILE:${PROJECT_NAME}-test>")

# std::thread depends on thread
target_link_libraries(${PROJECT_NAME} "Threads::Threads")
target_link_libraries(${PROJECT_NAME}-test "Threads::Threads")

add_dependencies(${PROJECT_NAME} JSON11)
target_link_libraries(${PROJECT_NAME} "json11")
add_dependencies(${PROJECT_NAME}-test GTEST JSON11)
target_link_libraries(${PROJECT_NAME}-test "gtest_main" "gtest" "json11")

# install
install(TARGETS ${PROJECT_NAME}
	RUNTIME DESTINATION "bin")
