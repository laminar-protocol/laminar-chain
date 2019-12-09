#!/usr/bin/env bash

VERSION=$1

if [[ -z "$1" ]] ; then
    echo "Usage: ./scripts/docker-hub-publish.sh VERSION"
    exit 1
fi

docker build . -t flowchain/flowchain-node:$1 -t flowchain/flowchain-node:latest
docker push flowchain/flowchain-node:$1
docker push flowchain/flowchain-node:latest
