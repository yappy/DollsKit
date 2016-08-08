.PHONY: default all nuget build deploy clean

NUGET_DIR = packages
DEPLOY_DIR = deploy
DEPLOY_FILES += Shanghai/bin/Release/*.exe*
DEPLOY_FILES += Shanghai/bin/Release/*.dll

default: build deploy

all: nuget build deploy

nuget:
	nuget install -OutputDirectory $(NUGET_DIR) Shanghai/packages.config

build:
	xbuild DollsKit.sln /p:Configuration=Debug
	xbuild DollsKit.sln /p:Configuration=Release

deploy:
	mkdir -p $(DEPLOY_DIR)
	cp $(DEPLOY_FILES) $(DEPLOY_DIR)

clean:
#	rm -rf $(NUGET_DIR)
	xbuild DollsKit.sln /t:Clean /p:Configuration=Debug
	xbuild DollsKit.sln /t:Clean /p:Configuration=Release
