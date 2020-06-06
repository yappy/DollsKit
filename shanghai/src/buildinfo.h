#ifndef SHANGHAI_BUILDINFO_H
#define SHANGHAI_BUILDINFO_H

// buildinfo_cmake.h を安全のため毎回生成するが、ビルド時間短縮のため
// buildinfo.cpp のみのリビルドに留める
namespace shanghai::buildinfo {

const char *BuildType();
const char *GitBranch();
const char *GitHash();

}	// shanghai::buildinfo

#endif	// SHANGHAI_BUILDINFO_H
