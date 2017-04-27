extern crate fish_in_a_jit as fj;

use fj::dmo::Dmo;
use fj::bytecode::Bytecode;
use fj::jit::{JitMemory, JitAssembler};

fn main() {
    let text = r#"
operators:
  - Draw: [ 0, 2, 1.5 ]
  - Print

context:
  sprites:
    - " ><(([°> "
"#;

    let d = Dmo::new_from_yml_str(&text).unwrap();
    let bytecode = d.to_bytecode();
    let mut dmo = Dmo::from_bytecode(bytecode);

    let mut jm: JitMemory = JitMemory::new(1);

    jm.fill_jit(&mut dmo.context, &dmo.operators);
    let jit_fn = jm.to_jit_fn();

    print!("\n");
    jit_fn.run(&mut dmo.context);
    print!("\n");
}
