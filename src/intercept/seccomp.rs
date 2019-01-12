extern crate seccomp_sys;
use seccomp_sys::*;

use crate::err::{Error, Result};

/// `seccomp` context to which rules are applied
pub struct Context {
    ctx: *mut scmp_filter_ctx,
}

impl Context {
    /// Create new `seccomp` context
    pub fn new() -> Result<Self> {
        let ctx = unsafe { seccomp_init(SCMP_ACT_ALLOW) };
        if ctx.is_null() {
            Err(Error::Seccomp {
                src: "seccomp_init returned null",
            })
        } else {
            Ok(Context { ctx })
        }
    }

    /// Add `seccomp` rule to trace syscall `call`
    pub fn trace(self, call: i32) -> Result<Self> {
        let ret = unsafe { seccomp_rule_add(self.ctx, SCMP_ACT_TRACE(0), call, 0) };
        if ret != 0 {
            Err(Error::Seccomp {
                src: "seccomp_rule_add returned error",
            })
        } else {
            Ok(self)
        }
    }

    /// Load the created `seccomp` filter
    pub fn load(self) -> Result<()> {
        let ret = unsafe { seccomp_load(self.ctx) };
        if ret != 0 {
            Err(Error::Seccomp {
                src: "seccomd_load returned error",
            })
        } else {
            Ok(())
        }
    }
}

impl Drop for Context {
    /// Release context on drop
    fn drop(&mut self) {
        unsafe { seccomp_release(self.ctx) };
    }
}
