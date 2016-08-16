.PHONY: default all nuget build clean

NUGET_DIR = packages

default: build deploy

all: nuget build deploy

nuget:
	nuget install -OutputDirectory $(NUGET_DIR) Shanghai/packages.config
	nuget install -OutputDirectory $(NUGET_DIR) DLearn/packages.config

build:
	xbuild DollsKit.sln /p:Configuration=Debug
	xbuild DollsKit.sln /p:Configuration=Release

clean:
#	rm -rf $(NUGET_DIR)
	xbuild DollsKit.sln /t:Clean /p:Configuration=Debug
	xbuild DollsKit.sln /t:Clean /p:Configuration=Release
