#!/bin/sh
# Build json schemas for every contract
SRC=$(git rev-parse --show-toplevel)
cd $SRC

for f in contracts/*; do
  sh -d
  if [ -d "$f" ]; then
    cd $f
    cargo schema
    cd $SRC
  fi
done

