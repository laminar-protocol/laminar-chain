#!/usr/bin/env bash

VERSION=$1

if [[ -z "$1" ]] ; then
    echo "Usage: ./scripts/docker-hub-publish.sh VERSION"
    exit 1
fi

docker build . -t laminar/laminar-node:$1 -t laminar/laminar-node:latest --no-cache
docker push laminar/laminar-node:$1
docker push laminar/laminar-node:latest
