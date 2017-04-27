use std::fs::File;
use std::path::PathBuf;
use std::error::Error;
use std::io::Write;

use serde_yaml;

use bytecode::Bytecode;

pub const BUFFER_SIZE: usize = 50;

/// Holds the data we need to access when running the code
#[derive(Serialize, Deserialize)]
pub struct Dmo {
    pub context: Context,
    pub operators: Vec<Operator>,
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

/// Represents instructions for building the JIT fn. We will iterate over a
/// `Vec<Operator>`.
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
}

impl Dmo {
    pub fn new() -> Dmo {
        Dmo::default()
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
}
