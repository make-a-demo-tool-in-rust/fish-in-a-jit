extern crate fish_in_a_jit as fj;

use std::thread::sleep;
use std::time::Duration;

use fj::dmo::Dmo;
use fj::bytecode::Bytecode;
use fj::jit::{JitMemory, JitAssembler};

pub fn main() {
    // Read in at compile time, include path is relative to the .rs file.
    let bytecode = include_bytes!("./fish-demo.dmo").to_vec();

    let mut dmo = Dmo::from_bytecode(bytecode);

    let mut jm: JitMemory = JitMemory::new(1);
    jm.fill_jit(&mut dmo.context, &dmo.operators);
    let jit_fn = jm.to_jit_fn();

    print!("\n");

    while dmo.context.is_running {
        jit_fn.run(&mut dmo.context);

        sleep(Duration::from_millis(10));
        dmo.context.time += 0.01;
    }

    print!("\n");
}
