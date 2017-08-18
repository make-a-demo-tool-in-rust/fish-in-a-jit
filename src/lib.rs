#![feature(link_llvm_intrinsics)]
#![feature(abi_sysv64)]
#![feature(try_from)]
#![allow(dead_code)]

#![cfg(all(any(target_os = "linux", target_os = "macos", target_os = "windows"), target_arch = "x86_64"))]

#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

#[cfg(any(target_os = "linux", target_os = "macos"))]
extern crate libc;

#[cfg(target_os = "windows")]
extern crate winapi;
#[cfg(target_os = "windows")]
extern crate kernel32;

pub mod dmo;
pub mod bytecode;
pub mod jit;
pub mod utils;

pub mod tests;
