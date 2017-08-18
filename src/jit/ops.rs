use dmo::Context;

pub trait Ops {
    extern "sysv64" fn op_print(&self);
    extern "sysv64" fn op_exit(&mut self, limit: f32);
    extern "sysv64" fn op_draw(&mut self, sprite_idx: u8, offset: u8, speed: f32);
    extern "sysv64" fn op_clear(&mut self, charcode: u32);
}

impl Ops for Context {
    extern "sysv64" fn op_print(&self) {
        self.impl_print();
    }

    extern "sysv64" fn op_exit(&mut self, limit: f32) {
        self.impl_exit(limit);
    }

    extern "sysv64" fn op_draw(&mut self, sprite_idx: u8, offset: u8, speed: f32) {
        self.impl_draw(sprite_idx, offset, speed);
    }

    extern "sysv64" fn op_clear(&mut self, charcode: u32) {
        self.impl_clear(charcode);
    }
}
