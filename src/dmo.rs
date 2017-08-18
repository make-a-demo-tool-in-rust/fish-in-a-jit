use std::str;
use std::fs::File;
use std::path::PathBuf;
use std::error::Error;
use std::io::Write;
use std::convert::TryFrom;

use serde_yaml;

use jit::JitFn;
use bytecode::Bytecode;

pub const BUFFER_SIZE: usize = 50;

/// Holds the data we need to access when running the code.
///
/// The `Context` and `Vec<Operator>` are private to make them only accessible
/// through API calls which should remember to rebuild the `JitFn` as well.
#[derive(Serialize, Deserialize)]
pub struct Dmo {
    context: Context,
    operators: Vec<Operator>,

    #[serde(skip_serializing, skip_deserializing)]
    jit_fn: JitFn,
}

#[derive(Serialize, Deserialize)]
pub struct Context {
    pub sprites: Vec<String>,

    #[serde(skip_serializing, skip_deserializing)]
    pub buffer: Vec<char>,
    #[serde(skip_serializing, skip_deserializing)]
    pub is_running: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub time: f32,
}

/// Represents instructions which are executed by the JIT fn, which is assembled
/// while iterating over a `Vec<Operator>`.
#[derive(Serialize, Deserialize)]
pub enum Operator {
    /// No operation
    NOOP,
    /// Exit the main loop if time is greater than this value
    Exit(f32),
    /// Print the text buffer
    Print,
    /// Draw a sprite into the buffer: sprite idx, offset, time speed
    Draw(u8, u8, f32),
    /// Clear the text buffer with a character code, expect UTF-32 unicode
    Clear(u32),
}

impl Default for Dmo {
    fn default() -> Dmo {
        Dmo {
            context: Context::default(),
            operators: vec![],
            jit_fn: JitFn::default(),
        }
    }
}

impl Default for Context {
    fn default() -> Context {
        Context {
            sprites: vec![],
            buffer: ['_'; BUFFER_SIZE].to_vec(),
            is_running: true,
            time: 0.0,
        }
    }
}

impl Context {
    pub fn new() -> Context {
        Context::default()
    }

    /// Prints the text buffer, followed by a `\r` (rewind)
    pub fn impl_print(&self) {
        let s: String = self.buffer.iter().cloned().collect();
        let text = format!("     {}\r", s);
        print!("{}", text)
    }

    /// `.is_running` is the break condition for the main drawing loop. This
    /// sets `.is_running` to `false` if `.time` is over the `limit`.
    pub fn impl_exit(&mut self, limit: f32) {
        if self.time > limit {
            self.is_running = false;
        }
    }

    /// Write a text sprite into the buffer, starting at `offset` and moving
    /// with `speed`.
    pub fn impl_draw(&mut self, sprite_idx: u8, offset: u8, speed: f32) {
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

    /// Clear the buffer by filling it with a character code.
    pub fn impl_clear(&mut self, charcode: u32) {
        let ch = TryFrom::try_from(charcode).unwrap();
        for i in 0 .. self.buffer.len() {
            self.buffer[i] = ch;
        }
    }
}

impl Dmo {
    /// Two steps are necessary. After `new()`, call `.build_jit_fn()`.
    pub fn new(context: Context, operators: Vec<Operator>) -> Dmo {
        Dmo {
            context: context,
            operators: operators,
            jit_fn: JitFn::default(),
        }
    }

    /// This must happen after `dmo` is assigned, so that the JIT is
    /// built with the pointer address of the new `dmo.context`.
    pub fn build_jit_fn(&mut self) {
        self.jit_fn = JitFn::new(1, &mut self.context, &self.operators);
    }

    pub fn run_jit_fn(&mut self) {
        self.jit_fn.run(&mut self.context)
    }

    pub fn new_from_yml_str(text: &str) -> Result<Dmo, Box<Error>> {
        let dmo: Dmo = try!(serde_yaml::from_str(text));
        Ok(dmo)
    }

    pub fn write_to_blob(&self, path: &PathBuf) -> Result<(), Box<Error>> {
        let mut f: File = try!(File::create(&path));
        let bytecode = self.to_bytecode();
        f.write_all(bytecode.as_slice()).unwrap();
        Ok(())
    }

    pub fn get_sprites(&self) -> &Vec<String> {
        &self.context.sprites
    }

    pub fn get_operators(&self) -> &Vec<Operator> {
        &self.operators
    }

    pub fn get_is_running(&self) -> bool {
        self.context.is_running
    }

    pub fn add_to_time(&mut self, add: f32) {
        self.context.time += add
    }
}
