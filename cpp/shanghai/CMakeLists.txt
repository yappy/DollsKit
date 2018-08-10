cmake_minimum_required(VERSION 3.7)

project(shanghai)

file(GLOB SRCS "src/*.cpp")
add_executable(${PROJECT_NAME} ${SRCS})

set(THREADS_PREFER_PTHREAD_FLAG ON)
find_package(Threads REQUIRED)
target_link_libraries(${PROJECT_NAME} "Threads::Threads")

install(TARGETS ${PROJECT_NAME}
	RUNTIME DESTINATION "bin")
