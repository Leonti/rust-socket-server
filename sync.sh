#!/usr/bin/env bash

cargo build --target=armv7-unknown-linux-musleabihf --release

mkdir -p target/armv7-unknown-linux-musleabihf/release/app
cp target/armv7-unknown-linux-musleabihf/release/rover_server target/armv7-unknown-linux-musleabihf/release/app/rover_server

balena sync d206b55 --source target/armv7-unknown-linux-musleabihf/release/app --destination /app
