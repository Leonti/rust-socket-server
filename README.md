# rust-socket-server

```
sudo apt-get install libudev-dev
```
Cross-compiling:
https://github.com/dcuddeback/libudev-sys

`~/.cargo/config`
```
[target.armv7-unknown-linux-musleabihf]
linker = "arm-linux-gnueabihf-gcc-8"
```

```
rustup target add armv7-unknown-linux-musleabihf
sudo apt-get install gcc-8-multilib-arm-linux-gnueabihf
cargo build --target=armv7-unknown-linux-musleabihf
```
https://github.com/japaric/rust-cross
sudo apt install gcc-arm-linux-gnueabihf
https://docs.rs/crate/serialport/3.0.0/source/.gitlab-ci.yml


https://github.com/pirogoeth/rust-pca9685


## Compiling on OSX
https://grahamenos.com/rust-osx-linux-musl.html

`brew install FiloSottile/musl-cross/musl-cross`

`.cargo/config`
```
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
```

`rustup target add x86_64-unknown-linux-musl`
`cargo build --target=x86_64-unknown-linux-musl`  or
`cargo check --target=x86_64-unknown-linux-musl`

```
Received sensor message, broadcasting: "{\"generic\":{\"message\":\"Compass sensor message\"}}"
Received sensor message, broadcasting: "{\"generic\":{\"message\":\"Axl sensor message\"}}"
Received sensor message, broadcasting: "{\"arduino\":{\"event\":{\"power\":{\"load_voltage\":2.28,\"current_ma\":0.1}}}}"
```
