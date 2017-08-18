extern crate fish_in_a_jit as fj;

use std::thread::sleep;
use std::time::Duration;

use fj::dmo::Dmo;
use fj::bytecode::Bytecode;

pub fn main() {
    // Read in at compile time, include path is relative to the .rs file.
    let bytecode = include_bytes!("./fish-demo.dmo").to_vec();

    let mut dmo = Dmo::from_bytecode(bytecode);
    dmo.build_jit_fn();

    print!("\n");

    while dmo.get_is_running() {
        dmo.run_jit_fn();
        sleep(Duration::from_millis(10));
        dmo.add_to_time(0.01);
    }

    print!("\n");
}
