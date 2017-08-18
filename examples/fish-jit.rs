extern crate fish_in_a_jit as fj;

use std::path::PathBuf;

use std::thread::sleep;
use std::time::Duration;

use fj::dmo::Dmo;
use fj::bytecode::Bytecode;
use fj::utils::file_to_string;

pub fn main() {
    // Read in at runtime, include path is relative to current directory, which
    // is the crate root if running this example with "cargo run --example fish-jit".
    let text = file_to_string(&PathBuf::from("./examples/fish-demo.yml")).unwrap();
    let d = Dmo::new_from_yml_str(&text).unwrap();
    let bytecode = d.to_bytecode();

    // Write the bytecode blob while we are at it, for the standalone example to
    // use with include_bytes!()
    d.write_to_blob(&PathBuf::from("./examples/fish-demo.dmo")).unwrap();

    // A Dmo from the bytecode directly, for testing that.
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
