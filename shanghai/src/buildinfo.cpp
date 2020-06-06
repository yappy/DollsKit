#include "buildinfo.h"
#include "buildinfo_cmake.h"

namespace shanghai::buildinfo {

const char *BuildType() { return BUILD_INFO_BUILD_TYPE; }
const char *GitBranch() { return BUILD_INFO_GIT_BRANCH; }
const char *GitHash() { return BUILD_INFO_GIT_HASH; }

}	// shanghai::buildinfo
