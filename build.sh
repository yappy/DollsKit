#!/bin/sh

nuget install -OutputDirectory packages Shanghai/packages.config
xbuild DollsKit.sln /p:Configuration=Debug
xbuild DollsKit.sln /p:Configuration=Release
