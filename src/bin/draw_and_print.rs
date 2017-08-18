extern crate fish_in_a_jit as fj;

use fj::dmo::Dmo;
use fj::bytecode::Bytecode;

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
    dmo.build_jit_fn();

    print!("\n");
    dmo.run_jit_fn();
    print!("\n");
}
