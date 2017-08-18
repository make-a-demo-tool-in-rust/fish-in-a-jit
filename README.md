Fish in a JIT

Demonstration of Just-In-Time compilation for x86_64 arch on Linux, Windows and Mac OS.

Code for Chapter 1 in 'Make a Demo Tool in Rust'

http://make-a-demo-tool-in-rust.github.io

Requires `nightly` Rust, last time I checked `7ac979d8c 2017-08-16` worked.

This is a simple enough project, but if you have to compile it on a particular
Rust version:

``` bash
rustup install nightly-2017-08-16
rustup default nightly-2017-08-16-x86_64-unknown-linux-gnu
```

Then run the examples:

```
cargo run --bin draw_and_print 
cargo run --example fish-jit
cargo run --example fish-standalone
```
