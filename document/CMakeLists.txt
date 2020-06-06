cmake_minimum_required(VERSION 3.7)

project(document)

set(DOC_INSTALL_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../docs")
set(GRAPHVIS_FORMAT "png")

# all
file(GLOB_RECURSE SRCS RELATIVE "${CMAKE_CURRENT_SOURCE_DIR}" "src/*")
# others
set(SRCS_OTHER ${SRCS})
# Graphviz source
file(GLOB_RECURSE SRCS_DOT RELATIVE "${CMAKE_CURRENT_SOURCE_DIR}" "src/*.dot")
list(REMOVE_ITEM SRCS_OTHER ${SRCS_DOT})

set(BIN_LIST)
set(INST_LIST)

# Graphviz: dot
foreach(ELEM ${SRCS_DOT})
	set(SRC  "${CMAKE_CURRENT_SOURCE_DIR}/${ELEM}")
	get_filename_component(DIR ${ELEM} DIRECTORY)
	get_filename_component(WE ${ELEM} NAME_WE)
	set(DEST "${CMAKE_CURRENT_BINARY_DIR}/${DIR}/${WE}.${GRAPHVIS_FORMAT}")
	get_filename_component(DEST_DIR ${DEST} DIRECTORY)
	add_custom_command(
		OUTPUT ${DEST}
		COMMAND "mkdir" "-p" ${DEST_DIR}
		COMMAND "dot" "-T${GRAPHVIS_FORMAT}" "-o${DEST}" ${SRC}
		DEPENDS ${SRC})
	list(APPEND BIN_LIST ${DEST})
endforeach()
# others: copy
foreach(ELEM ${SRCS_OTHER})
	set(SRC  "${CMAKE_CURRENT_SOURCE_DIR}/${ELEM}")
	set(DEST "${CMAKE_CURRENT_BINARY_DIR}/${ELEM}")
	get_filename_component(DEST_DIR ${DEST} DIRECTORY)
	add_custom_command(
		OUTPUT ${DEST}
		COMMAND "mkdir" "-p" ${DEST_DIR}
		COMMAND "cp" ${SRC} ${DEST}
		DEPENDS ${SRC})
	list(APPEND BIN_LIST ${DEST})
endforeach()

#foreach(ELEM ${BIN_LIST})
#	set(SRC  ${ELEM})
#	file(RELATIVE_PATH REL "${CMAKE_CURRENT_BINARY_DIR}/src" ${ELEM})
#	set(DEST "${DOC_INSTALL_DIR}/${REL}")
#	get_filename_component(DEST_DIR ${DEST} DIRECTORY)
#	add_custom_command(
#		OUTPUT ${DEST}
#		COMMAND "mkdir" "-p" ${DEST_DIR}
#		COMMAND "cp" ${SRC} ${DEST}
#		DEPENDS ${SRC})
#	list(APPEND INST_LIST ${DEST})
#endforeach()

# make doc
add_custom_target("doc"
	SOURCES ${BIN_LIST})
add_custom_target("doc_update"
	DEPENDS "doc_git_add")

add_custom_target("doc_git_add"
	COMMAND "git" "add" "${DOC_INSTALL_DIR}"
	COMMAND "git" "status"
	DEPENDS "doc_install")
add_custom_target("doc_install"
	DEPENDS "doc"
	DEPENDS "doc_git_clean"
	SOURCES ${INST_LIST})
add_custom_target("doc_git_clean"
	COMMAND "git" "rm" "--ignore-unmatch" "-f" "${DOC_INSTALL_DIR}/?*.*"
	COMMAND "git" "clean" "-fxd" "${DOC_INSTALL_DIR}"
	COMMAND "git" "status")
