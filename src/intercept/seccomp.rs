extern crate seccomp_sys;
use seccomp_sys::*;

pub struct Context {
    ctx: *mut scmp_filter_ctx,
}

impl Context {
    pub fn new() -> Self {
        let ctx = unsafe { seccomp_init(SCMP_ACT_ALLOW) };
        if ctx.is_null() {
            // TODO: Remove
            panic!("seccomp_init returned null");
        }

        Context { ctx }
    }

    pub fn trace(self, call: i32) -> Self {
        let ret = unsafe { seccomp_rule_add(self.ctx, SCMP_ACT_TRACE(0), call, 0) };
        if ret != 0 {
            panic!("seccomp_rule_add returned error");
        }

        self
    }

    pub fn load(self) {
        let ret = unsafe { seccomp_load(self.ctx) };
        if ret != 0 {
            panic!("seccomd_load returned error");
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { seccomp_release(self.ctx) };
    }
}
