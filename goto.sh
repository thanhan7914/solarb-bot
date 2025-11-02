#!/bin/bash

# container name
CONTAINER_NAME="solarb-bot"
CONTAINER_ID=$(docker ps --filter "name=$CONTAINER_NAME" | awk 'NR>1 {print $1}')

if [ -z "$CONTAINER_ID" ]; then
  echo "Not found container name '$CONTAINER_NAME'"
  exit 1
fi
echo "Exec into container ID: $CONTAINER_ID"
docker exec -it "$CONTAINER_ID" bash
