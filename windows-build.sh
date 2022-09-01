#! /bin/bash
cargo install cross
rustup target add x86_64-pc-windows-gnu
cross build --target=x86_64-pc-windows-gnu

cross build --target=x86_64-pc-windows-msvc