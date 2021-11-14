#!/usr/bin/env sh

for file in libs/*
do
    realpath=`readlink -f libs/libdjvulibre.so.21`
    directory=`dirname "$realpath"`
    filename=`basename "$file"`
    ln -s "$filename" "$directory/${filename%.so*}.so"
done
