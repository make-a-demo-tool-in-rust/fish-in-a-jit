#[cfg(all(any(target_os = "linux", target_os = "macos"), target_arch = "x86_64"))]

use std::str;
use std::convert::TryFrom;
use dmo::Context;

pub trait Ops {
    extern "sysv64" fn print(&self);
    extern "sysv64" fn exit(&mut self, limit: f32);
    extern "sysv64" fn draw(&mut self, sprite_idx: u8, offset: u8, speed: f32);
    extern "sysv64" fn clear(&mut self, charcode: u32);
}

impl Ops for Context {

    /// Prints the text buffer, followed by a `\r` rewind
    extern "sysv64" fn print(&self) {
        let s: String = self.buffer.iter().cloned().collect();
        let text = format!("     {}\r", s);
        print!("{}", text)
    }

    /// We use `.is_running` as the break condition for the main drawing loop.
    /// This sets `.is_running` to `false` if `.time` is over the `limit`.
    extern "sysv64" fn exit(&mut self, limit: f32) {
        if self.time > limit {
            self.is_running = false;
        }
    }

    extern "sysv64" fn draw(&mut self, sprite_idx: u8, offset: u8, speed: f32) {
        if (sprite_idx as usize) < self.sprites.len() {

            let total_offset: usize = ((offset as f32 + self.time * speed) % (self.buffer.len() as f32)) as usize;

            let ref sprite = self.sprites[sprite_idx as usize];

            let mut v: Vec<char> = vec![];
            for ch in sprite.chars() { v.push(ch); }

            for i in 0 .. v.len() {
                let n = (total_offset + i) % self.buffer.len();
                self.buffer[n] = v[i];
            }
        }
    }

    extern "sysv64" fn clear(&mut self, charcode: u32) {
        let ch = TryFrom::try_from(charcode).unwrap();
        for i in 0 .. self.buffer.len() {
            self.buffer[i] = ch;
        }
    }
}
