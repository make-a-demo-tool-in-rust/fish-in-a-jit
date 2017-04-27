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

    let cmd = Command::new(path).output().expect("failed to execute process");

    let text = String::from("\n     __ ><(([°> _______________________________________\r\n");

    let output = String::from_utf8(cmd.stdout).unwrap();

    println!("{}", output);

    assert_eq!(text, output);
}
