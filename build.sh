#!/bin/bash
mkdir -p target/builds
cargo build --target x86_64-pc-windows-gnu
mv target/x86_64-pc-windows-gnu/debug/chaos.exe target/builds/chaos-debug-x86_64-pc-windows-gnu.exe
cargo build --target x86_64-apple-darwin
mv target/x86_64-apple-darwin/debug/chaos target/builds/chaos-debug-x86_64-apple-darwin
cargo build --target aarch64-apple-darwin
mv target/aarch64-apple-darwin/debug/chaos target/builds/chaos-debug-aarch64-apple-darwin
