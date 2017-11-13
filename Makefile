.PHONY: default all nuget build clean

NUGET_DIR = packages

default: all

all: nuget build

nuget:
	nuget restore DollsKit.sln

build:
	msbuild DollsKit.sln /p:Configuration=Debug
	msbuild DollsKit.sln /p:Configuration=Release

clean:
	msbuild DollsKit.sln /t:Clean /p:Configuration=Debug
	msbuild DollsKit.sln /t:Clean /p:Configuration=Release
	rm -rf $(NUGET_DIR)
