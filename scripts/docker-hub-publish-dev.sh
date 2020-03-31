#!/usr/bin/env bash

VERSION=$(git rev-parse --short HEAD)

docker build . -t laminardev/laminar-node:$VERSION --no-cache
docker push laminardev/laminar-node:$VERSION
