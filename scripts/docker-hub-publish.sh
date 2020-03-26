#!/usr/bin/env bash

VERSION=$1

if [[ -z "$1" ]] ; then
    echo "Usage: ./scripts/docker-hub-publish.sh VERSION"
    exit 1
fi

docker build . -t laminardev/laminar-node:$1 -t laminardev/laminar-node:latest --no-cache
docker push laminardev/laminar-node:$1
docker push laminardev/laminar-node:latest
