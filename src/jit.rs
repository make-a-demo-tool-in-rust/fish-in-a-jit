use std::mem;

#[cfg(target_os = "windows")]
use std::ptr;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use libc;

#[cfg(target_os = "windows")]
use winapi;
#[cfg(target_os = "windows")]
use kernel32;

use dmo::Operator as Op;
use dmo::Context;
use ops::Ops;

extern {
    // Because Ferris says it's good.
    #[link_name = "llvm.clear_cache"]
    pub fn clear_cache(a: *mut i8, b: *mut i8) -> ();
}

// Allocate memory sizes as multiples of 4k page.
const PAGE_SIZE: usize = 4096;

/// A read-write memory buffer allocated to be filled with bytes of `x86`
/// instructions.
pub struct JitMemory {
    addr: *mut u8,
    size: usize,
    /// current position for writing the next byte
    offset: usize,
}

/// An executable memory buffer filled with `x86` instructions.
pub struct JitFn {
    addr: *mut u8,
    size: usize,
}

impl JitFn {
    pub fn run(&self, context: &mut Context) {
        unsafe {
            // type signature of the jit function
            let fn_ptr: extern fn(&mut Context);
            // transmute the pointer of the executable memory to a pointer of the jit function
            fn_ptr = mem::transmute(self.addr);
            // use the function pointer
            fn_ptr(context)
        }
    }
}

pub trait JitAssembler {

    /// Marks the memory block as executable and returns a `JitFn` containing
    /// that address.
    fn to_jit_fn(&mut self) -> JitFn;

    /// Fills the memory block with `x86` instructions while iterating over a
    /// list of `Operator` enums.
    fn fill_jit(&mut self, context: &mut Context, operators: &Vec<Op>);

    /// Writes one byte to the memory at the current index offset and increments
    /// the offset.
    fn push_u8(&mut self, value: u8);

    /// Writes a 4-byte value. `x86` specifies Little-Endian encoding,
    /// least-significant byte first.
    fn push_u32(&mut self, value: u32);

    /// Writes an 8-byte value.
    fn push_u64(&mut self, value: u64);
}

impl JitMemory {

    // Memory must be aligned on a boundary of multiples of 16 bytes to work on
    // Win / Mac / Linux.
    //
    // See LIBC, 3.2.3.6 Allocating Aligned Memory Blocks:
    //
    // Function: int posix_memalign (void **memptr, size_t alignment, size_t size)

    /// Allocates read-write memory aligned on a 16 byte boundary.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn new(num_pages: usize) -> JitMemory {
        let size: usize = num_pages * PAGE_SIZE;
        let addr: *mut u8;

        unsafe {
            // Take a pointer
            let mut raw_addr: *mut libc::c_void = mem::uninitialized();

            // Allocate aligned to page size
            libc::posix_memalign(&mut raw_addr,
                                 PAGE_SIZE,
                                 size);

            // Mark the memory as read-write
            libc::mprotect(raw_addr,
                           size,
                           libc::PROT_READ | libc::PROT_WRITE);

            // Fill with 'RET' calls (0xc3)
            libc::memset(raw_addr, 0xc3, size);

            // Transmute the c_void pointer to a Rust u8 pointer
            addr = mem::transmute(raw_addr);
        }

        JitMemory {
            addr: addr,
            size: size,
            offset: 0,
        }
    }

    #[cfg(target_os = "windows")]
    pub fn new(num_pages: usize) -> JitMemory {
        let size: usize = num_pages * PAGE_SIZE;
        let addr: *mut u8;

        unsafe {
            // Take a pointer
            let raw_addr: *mut winapi::c_void;

            // VirtualAlloc(lpAddress: LPVOID, dwSize: SIZE_T, flAllocationType: DWORD, flProtect: DWORD) -> LPVOID

            // Allocate aligned to page size
            raw_addr = kernel32::VirtualAlloc(
                ptr::null_mut(),
                size as u64,
                winapi::MEM_RESERVE | winapi::MEM_COMMIT,
                winapi::winnt::PAGE_READWRITE);

            if raw_addr == 0 as *mut winapi::c_void {
                panic!("Couldn't allocate memory.");
            }

            // NOTE no FillMemory() or SecureZeroMemory() in kernel32

            // Transmute the c_void pointer to a Rust u8 pointer
            addr = mem::transmute(raw_addr);

        }

        JitMemory {
            addr: addr,
            size: size,
            offset: 0,
        }
    }

    pub fn get_addr(&self) -> *mut u8 {
        self.addr
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }
}

impl JitAssembler for JitMemory {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn to_jit_fn(&mut self) -> JitFn {
        unsafe {
            libc::mprotect(
                self.addr as *mut _,
                self.size,
                libc::PROT_READ | libc::PROT_EXEC,
            );

            clear_cache(self.addr as *mut _, (self.addr as *mut _).offset(self.size as _));
        }

        JitFn {
            addr: self.addr,
            size: self.size,
        }
    }

    #[cfg(target_os = "windows")]
    fn to_jit_fn(&mut self) -> JitFn {
        // VirtualProtect(lpAddress: LPVOID, dwSize: SIZE_T, flNewProtect: DWORD, lpflOldProtect: DWORD) -> BOOL
        unsafe {
            let old_prot: *mut winapi::DWORD = mem::uninitialized();
            kernel32::VirtualProtect(
                self.addr as *mut _,
                self.size as u64,
                winapi::winnt::PAGE_EXECUTE_READ,
                old_prot as *mut _,
            );

            clear_cache(self.addr as *mut _, (self.addr as *mut _).offset(self.size as _));
        }

        JitFn {
            addr: self.addr,
            size: self.size,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn fill_jit(&mut self, context: &mut Context, operators: &Vec<Op>) {
        // prologue
        self.push_rbp();
        self.mov_rbp_rsp();

        for op in operators.iter() {
            match *op {
                Op::NOOP => (),

                Op::Exit(limit) => {
                    // x86_64 ABI is sysv64, arguments are passed in registers, and
                    // remaining ones are passed on the stack.
                    //
                    // Integer arguments are passed in rdi, rsi, rdx, rcx, etc.
                    //
                    // Floating-point arguments are passed in xmm0 - xmm7.

                    // rdi: pointer to Context (pointer is an integer value)
                    self.movabs_rdi_u64( unsafe { mem::transmute(context as *mut _) });

                    // xmm0: limit argument (floating point)
                    self.movss_xmm_n_f32(0, limit);

                    // put the address of the function in rax
                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::exit as extern "sysv64" fn(&mut Context, f32)
                    ) });

                    // rsp must be aligned on a 16-byte boundary before the call
                    // jump. Remember that call will push rdi, moving rsp with -8
                    // bytes immediately before the jump.
                    //
                    // We made 1 push (rsp -8 bytes), and call will add another push
                    // (-8 again), which is -16, so we don't have to sub any more.

                    // call the function address in rax
                    self.call_rax();

                    // It is good to note that if we didn't have enough registers
                    // for the function arguments, we would have had to push the
                    // remaining ones to the stack, and here we would have to clean
                    // up (the stack) by adding to rsp.
                    //
                    // But there is no cleaning to do at this time.
                },

                Op::Print => {
                    self.movabs_rdi_u64( unsafe { mem::transmute(context as *mut _) });
                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::print as extern "sysv64" fn(&Context)
                    )});
                    self.call_rax();
                },

                Op::Draw(sprite_idx, offset, speed) => {
                    // rdi: pointer to Context (pointer is an integer value)
                    self.movabs_rdi_u64( unsafe { mem::transmute(context as *mut _) });
                    // rsi: sprite_idx arg. (interger)
                    self.movabs_rsi_u64(sprite_idx as u64);
                    // rdx: offset arg. (interger)
                    self.movabs_rdx_u64(offset as u64);
                    // xmm0: speed arg. (floating point)
                    self.movss_xmm_n_f32(0, speed);

                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::draw as extern "sysv64" fn(&mut Context, u8, u8, f32)
                    )});
                    self.call_rax();
                },

                Op::Clear(charcode) => {
                    // rdi: pointer to Context (pointer is an integer value)
                    self.movabs_rdi_u64( unsafe { mem::transmute(context as *mut _) });
                    // rsi: char code (interger)
                    self.movabs_rsi_u64(charcode as u64);

                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::clear as extern "sysv64" fn(&mut Context, u32)
                    )});
                    self.call_rax();
                },
            }
        }

        // epilogue
        self.mov_rsp_rbp();
        self.pop_rbp();
        self.ret();
    }

    #[cfg(target_os = "windows")]
    fn fill_jit(&mut self, context: &mut Context, operators: &Vec<Op>) {
        // prologue
        // we just have to adjust for a 16 byte aligned rsp for call
        self.sub_rsp_u8(8);

        for op in operators.iter() {
            match *op {
                Op::NOOP => (),

                Op::Exit(limit) => {
                    self.movabs_rcx_u64( unsafe { mem::transmute(context as *mut _) });
                    self.movss_xmm_n_f32(1, limit);

                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::exit as extern "C" fn(&mut Context, f32)
                    ) });

                    self.call_rax();
                },

                Op::Print => {
                    self.movabs_rcx_u64( unsafe { mem::transmute(context as *mut _) });
                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::print as extern "C" fn(&Context)
                    )});
                    self.call_rax();
                },

                Op::Draw(sprite_idx, offset, speed) => {
                    self.movabs_rcx_u64( unsafe { mem::transmute(context as *mut _) });
                    self.movabs_rdx_u64(sprite_idx as u64);
                    self.movabs_r8_u64(offset as u64);
                    self.movss_xmm_n_f32(3, speed);

                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::draw as extern "C" fn(&mut Context, u8, u8, f32)
                    )});
                    self.call_rax();
                },

                Op::Clear(charcode) => {
                    self.movabs_rcx_u64( unsafe { mem::transmute(context as *mut _) });
                    self.movabs_rdx_u64(charcode as u64);

                    self.movabs_rax_u64( unsafe { mem::transmute(
                        Ops::clear as extern "C" fn(&mut Context, u32)
                    )});
                    self.call_rax();
                },
            }
        }

        // epilogue
        self.add_rsp_u8(8);
        self.ret();
    }

    fn push_u8(&mut self, value: u8) {
        unsafe { *self.addr.offset(self.offset as _) = value };
        self.offset += 1;
    }

    fn push_u32(&mut self, value: u32) {
        self.push_u8(((value >>  0) & 0xFF) as u8);
        self.push_u8(((value >>  8) & 0xFF) as u8);
        self.push_u8(((value >> 16) & 0xFF) as u8);
        self.push_u8(((value >> 24) & 0xFF) as u8);
    }

    fn push_u64(&mut self, value: u64) {
        self.push_u8(((value >>  0) & 0xFF) as u8);
        self.push_u8(((value >>  8) & 0xFF) as u8);
        self.push_u8(((value >> 16) & 0xFF) as u8);
        self.push_u8(((value >> 24) & 0xFF) as u8);
        self.push_u8(((value >> 32) & 0xFF) as u8);
        self.push_u8(((value >> 40) & 0xFF) as u8);
        self.push_u8(((value >> 48) & 0xFF) as u8);
        self.push_u8(((value >> 56) & 0xFF) as u8);
    }
}

impl JitMemory {

    pub fn ret(&mut self) {
        self.push_u8(0xc3);
    }

    pub fn mov_rax_u32(&mut self, value: u32) {
        self.push_u8(0x48);
        self.push_u8(0xc7);
        self.push_u8(0xc0);
        self.push_u32(value);
    }

    pub fn movabs_rax_u64(&mut self, value: u64) {
        self.push_u8(0x48);
        self.push_u8(0xb8);
        self.push_u64(value);
    }

    pub fn movabs_rdi_u64(&mut self, value: u64) {
        self.push_u8(0x48);
        self.push_u8(0xbf);
        self.push_u64(value);
    }

    pub fn movabs_rsi_u64(&mut self, value: u64) {
        self.push_u8(0x48);
        self.push_u8(0xbe);
        self.push_u64(value);
    }

    pub fn movabs_rdx_u64(&mut self, value: u64) {
        self.push_u8(0x48);
        self.push_u8(0xba);
        self.push_u64(value);
    }

    pub fn movabs_rcx_u64(&mut self, value: u64) {
        self.push_u8(0x48);
        self.push_u8(0xb9);
        self.push_u64(value);
    }

    pub fn movabs_r8_u64(&mut self, value: u64) {
        self.push_u8(0x49);
        self.push_u8(0xb8);
        self.push_u64(value);
    }

    pub fn movss_xmm_n_f32(&mut self, xmm_n: usize, value: f32) {
        // xmm0 - xmm7 are used to pass floating point arguments
        if xmm_n > 7 {
            return;
        }

        // pushq x
        self.push_u8(0x68);
        self.push_u32(unsafe { mem::transmute(value) });

        // movss xmm0, [rsp]
        self.push_u8(0xf3);// movss: 0xf3, movsd: 0xf2
        self.push_u8(0x0f);
        self.push_u8(0x10);

        match xmm_n {
            0 => {self.push_u8(0x04);// xmm0: 04 24
                  self.push_u8(0x24);},

            1 => {self.push_u8(0x0c);// xmm1: 0c 24
                  self.push_u8(0x24);},

            2 => {self.push_u8(0x14);// xmm2: 14 24
                  self.push_u8(0x24);},

            3 => {self.push_u8(0x1c);// xmm3: 1c 24
                  self.push_u8(0x24);},

            4 => {self.push_u8(0x24);// xmm4: 24 24
                  self.push_u8(0x24);},

            5 => {self.push_u8(0x2c);// xmm5: 2c 24
                  self.push_u8(0x24);},

            6 => {self.push_u8(0x34);// xmm6: 34 24
                  self.push_u8(0x24);},

            7 => {self.push_u8(0x3c);// xmm7: 3c 24
                  self.push_u8(0x24);},

            _ => {},
        }

        self.add_rsp_u8(8);
    }

    pub fn push_rax(&mut self) {
        self.push_u8(0x50);
    }

    pub fn call_rax(&mut self) {
        self.push_u8(0xff);
        self.push_u8(0xd0);
    }

    pub fn push_rbp(&mut self) {
        self.push_u8(0x55);
    }

    pub fn pop_rbp(&mut self) {
        self.push_u8(0x5d);
    }

    pub fn pop_rax(&mut self) {
        self.push_u8(0x58);
    }

    pub fn mov_rbp_rsp(&mut self) {
        self.push_u8(0x48);
        self.push_u8(0x89);
        self.push_u8(0xe5);
    }

    pub fn mov_rsp_rbp(&mut self) {
        self.push_u8(0x48);
        self.push_u8(0x89);
        self.push_u8(0xec);
    }

    pub fn add_rsp_u8(&mut self, value: u8) {
        self.push_u8(0x48);
        self.push_u8(0x83);
        self.push_u8(0xc4);
        self.push_u8(value);
    }

    pub fn sub_rsp_u8(&mut self, value: u8) {
        self.push_u8(0x48);
        self.push_u8(0x83);
        self.push_u8(0xec);
        self.push_u8(value);
    }
}

// NOTE: Could test if munmap return value is 0 if that's a concern for error
// handling.

// Function: void * mmap (void *address, size_t length, int protect, int flags, int filedes, off_t offset)

impl Drop for JitMemory {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn drop(&mut self) {
        unsafe { libc::munmap(self.addr as *mut _, self.size); }
    }

    #[cfg(target_os = "windows")]
    fn drop(&mut self) {
        unsafe { kernel32::VirtualFree(self.addr as *mut _, 0, winapi::MEM_RELEASE); }
    }
}

impl Drop for JitFn {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn drop(&mut self) {
        unsafe { libc::munmap(self.addr as *mut _, self.size); }
    }

    #[cfg(target_os = "windows")]
    fn drop(&mut self) {
        unsafe { kernel32::VirtualFree(self.addr as *mut _, 0, winapi::MEM_RELEASE); }
    }
}
