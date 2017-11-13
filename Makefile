.PHONY: default all build clean distclean distclean-force

NUGET_DIR = packages
CLEAN_EXCLUDE = -e "www/" -e "settings/"

default: all

all: build

build:
	nuget restore DollsKit.sln
	msbuild DollsKit.sln /p:Configuration=Debug
	msbuild DollsKit.sln /p:Configuration=Release

clean:
	msbuild DollsKit.sln /t:Clean /p:Configuration=Debug
	msbuild DollsKit.sln /t:Clean /p:Configuration=Release
	rm -rf $(NUGET_DIR)

distclean:
	git clean -nxd $(CLEAN_EXCLUDE)
	echo "'make distclean-force' if OK"

distclean-force:
	git clean -fxd $(CLEAN_EXCLUDE)
