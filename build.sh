#!/bin/sh
cargo +nightly build -Z build-std=std,core,alloc,panic_abort --release
