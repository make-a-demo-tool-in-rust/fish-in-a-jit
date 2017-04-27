#![cfg(test)]

use std::process::Command;

#[test]
fn draw_and_print() {

    // Path is relative to crate root.
    let path: &str = if cfg!(target_os = "windows") {
        "target\\debug\\draw_and_print.exe"
    } else {
        "./target/debug/draw_and_print"
    };

    let output = Command::new(path).output().expect("failed to execute process");

    let hello = output.stdout;

    println!("{:?}", hello);

    assert!(false);
}
