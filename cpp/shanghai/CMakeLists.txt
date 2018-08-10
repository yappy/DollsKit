cmake_minimum_required(VERSION 3.7)

project(shanghai)

file(GLOB SRCS "src/*.cpp")
add_executable(${PROJECT_NAME} ${SRCS})

# std::thread depends on thread
target_link_libraries(${PROJECT_NAME} "Threads::Threads")

add_dependencies(${PROJECT_NAME} JSON11)
target_link_libraries(${PROJECT_NAME} "json11")

# install
install(TARGETS ${PROJECT_NAME}
	RUNTIME DESTINATION "bin")
