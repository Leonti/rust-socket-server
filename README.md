# rust-socket-server

```
sudo apt-get install libudev-dev
```
Cross-compiling:
https://github.com/dcuddeback/libudev-sys  

`~/.cargo/config`  
```
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc-8"
```

```
rustup target add armv7-unknown-linux-gnueabihf
sudo apt-get install gcc-8-multilib-arm-linux-gnueabihf
export PKG_CONFIG_ALLOW_CROSS=1
cargo build --target=armv7-unknown-linux-gnueabihf
```
https://github.com/japaric/rust-cross
sudo apt install gcc-arm-linux-gnueabihf
https://docs.rs/crate/serialport/3.0.0/source/.gitlab-ci.yml
