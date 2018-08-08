cmake_minimum_required(VERSION 3.7)

project(shanghai)

file(GLOB SRCS "src/*.cpp")
add_executable(${PROJECT_NAME} ${SRCS})

install(TARGETS ${PROJECT_NAME}
	RUNTIME DESTINATION "bin")
