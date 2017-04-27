use std::{mem, ptr, str};
use std::convert::TryFrom;
use dmo::{Dmo, Context, Operator};

pub trait Bytecode {
    fn to_bytecode(&self) -> Vec<u8>;
    fn from_bytecode(data: Vec<u8>) -> Dmo;
}

impl Bytecode for Dmo {
    fn to_bytecode(&self) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();

        // Sprites
        // - u8: number of sprites
        // - u8: length of the sprite
        // - [u8]: sprite
        // ...

        res.push(self.context.sprites.len() as u8);

        // ASCII sprites can be unicode UTF-32, so expect the 4-byte char
        // instead of u8
        for sprite in self.context.sprites.iter() {
            // length in chars, not in bytes
            res.push(sprite.chars().count() as u8);

            let v: Vec<char> = sprite.chars().collect();
            for ch in v.iter() {
                push_u32(&mut res, *ch as u32)
            }
        }

        // === Operators ===

        // - u8: number of operators
        // - u8: opcode
        // - []: arguments of different types, but we always know how many and what kind there are

        res.push(self.operators.len() as u8);

        for op in self.operators.iter() {
            use dmo::Operator::*;
            match *op {
                NOOP => {},

                Exit(limit) => {
                    res.push(op_to_code(Exit(0.0)));
                    push_f32(&mut res, limit as f32);
                },

                Print => res.push(op_to_code(Print)),

                Draw(idx, offset, speed) => {
                    res.push(op_to_code(Draw(0, 0, 0.0)));

                    res.push(idx as u8);
                    res.push(offset as u8);
                    push_f32(&mut res, speed as f32);
                },

                Clear(charcode) => {
                    res.push(op_to_code(Clear(0)));
                    push_u32(&mut res, charcode as u32);
                },
            }
        }

        res
    }

    fn from_bytecode(data: Vec<u8>) -> Dmo {
        let mut blob = DataBlob::new(data);

        let mut context = Context::new();

        let mut n_sprites = blob.read_u8();

        while n_sprites >= 1 {
            // length of the sprite in chars, not in u8
            let l = blob.read_u8();

            let v = blob.read_char_vec(l as usize);
            let s: String = v.into_iter().collect();

            context.sprites.push(s);

            n_sprites -= 1;
        }

        // === Operators ===

        let mut operators: Vec<Operator> = vec![];

        let mut n_operators = blob.read_u8();

        while n_operators >= 1 {
            let op = code_to_op(blob.read_u8());

            use self::Operator::*;
            let op_val = match op {
                NOOP => NOOP,
                Exit(_) => Exit(blob.read_f32()),
                Print => Print,
                Draw(_, _, _) => {
                    Draw(blob.read_u8(),
                         blob.read_u8(),
                         blob.read_f32())
                },
                Clear(_) => Clear(blob.read_u32()),
            };

            match op_val {
                NOOP => {},
                _ => operators.push(op_val),
            }

            n_operators -= 1;
        }

        Dmo {
            context: context,
            operators: operators,
        }
    }
}

pub fn op_to_code(op: Operator) -> u8 {
    use dmo::Operator::*;
    match op {
        NOOP          => 0x00,
        Exit(_)       => 0x01,
        Draw(_, _, _) => 0x02,
        Clear(_)      => 0x03,
        Print         => 0xFF,
    }
}

pub fn code_to_op(code: u8) -> Operator {
    use dmo::Operator::*;
    match code {
        0x00 => NOOP,
        0x01 => Exit(0.0),
        0x02 => Draw(0, 0, 0.0),
        0x03 => Clear(0),
        0xFF => Print,
        _ => NOOP,
    }
}

pub struct DataBlob {
    data: Vec<u8>,
    idx: usize,
}

impl DataBlob {
    pub fn new(data: Vec<u8>) -> DataBlob {
        DataBlob {
            data: data,
            idx: 0,
        }
    }

    pub fn skip(&mut self, skip_len: usize) {
        self.idx += skip_len
    }

    pub fn read_u8(&mut self) -> u8 {
        let number = self.data[self.idx];
        self.idx += 1;
        number
    }

    pub fn read_u32(&mut self) -> u32 {
        let bytes: &[u8] = &self.data[self.idx .. self.idx+4];

        let mut number: u32 = 0;
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                &mut number as *mut u32 as *mut u8,
                4);
        };
        number.to_le();

        self.idx += 4;
        number
    }

    pub fn read_f32(&mut self) -> f32 {
        let number: f32 = unsafe { mem::transmute(self.read_u32()) };
        number
    }

    pub fn read_str(&mut self, str_len: usize) -> &str {
        if str_len == 0 {
            return "";
        }
        let text = str::from_utf8(&self.data[self.idx .. self.idx+str_len]).unwrap();
        self.idx += str_len;
        text
    }

    pub fn read_u8_vec(&mut self, str_len: usize) -> Vec<u8> {
        let mut text: Vec<u8> = Vec::new();

        text.extend(self.data[self.idx .. self.idx+str_len].iter().cloned());

        self.idx += str_len;
        text
    }

    pub fn read_char_vec(&mut self, str_len: usize) -> Vec<char> {
        let mut text: Vec<char> = Vec::new();

        for _ in 0 .. str_len {
            let n: u32 = self.read_u32();
            let ch: char = TryFrom::try_from(n).unwrap();
            text.push(ch);
        }

        text
    }
}

pub fn push_u32(mut v: &mut Vec<u8>, n: u32) {
    let bytes = unsafe { mem::transmute::<_, [u8; 4]>(n.to_le()) };
    v.push(bytes[0]);
    v.push(bytes[1]);
    v.push(bytes[2]);
    v.push(bytes[3]);
}

pub fn push_f32(mut v: &mut Vec<u8>, n: f32) {
    let val_u32: u32 = unsafe { mem::transmute(n) };
    push_u32(v, val_u32);
}

// NOTE: read_num_bytes and write_num_bytes macro in the byteorder crate by
// BurntSushi
//
// macro_rules! read_num_bytes {
//     ($ty:ty, $size:expr, $src:expr, $which:ident) => ({
//         assert!($size == ::core::mem::size_of::<$ty>());
//         assert!($size <= $src.len());
//         let mut data: $ty = 0;
//         unsafe {
//             copy_nonoverlapping(
//                 $src.as_ptr(),
//                 &mut data as *mut $ty as *mut u8,
//                 $size);
//         }
//         data.$which()
//     });
// }
//
// macro_rules! write_num_bytes {
//     ($ty:ty, $size:expr, $n:expr, $dst:expr, $which:ident) => ({
//         assert!($size <= $dst.len());
//         unsafe {
//             // N.B. https://github.com/rust-lang/rust/issues/22776
//             let bytes = transmute::<_, [u8; $size]>($n.$which());
//             copy_nonoverlapping((&bytes).as_ptr(), $dst.as_mut_ptr(), $size);
//         }
//     });
// }
