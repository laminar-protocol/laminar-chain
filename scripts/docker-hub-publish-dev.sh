#!/usr/bin/env bash

VERSION=$(git rev-parse --short HEAD)

docker build . -t laminardev/laminar-node:$VERSION -t laminardev/laminar-node:latest --no-cache
docker push laminardev/laminar-node:$VERSION
docker push laminardev/laminar-node:latest
