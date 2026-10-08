//! Tiny CPU helpers shared by both architectures (x86_64 and i686).
//!
//! `hlt`, `cli`/`sti`, `int3` and port I/O are the same instructions in
//! 32-bit and 64-bit mode, so the shared parts of the kernel (shell, apps,
//! timer, console, input) use these instead of an architecture crate.

use core::arch::asm;

#[inline(always)]
pub fn hlt() {
    unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) }
}

pub mod interrupts {
    use core::arch::asm;

    #[inline(always)]
    pub fn enable() {
        unsafe { asm!("sti", options(nomem, nostack)) }
    }
    #[inline(always)]
    pub fn disable() {
        unsafe { asm!("cli", options(nomem, nostack)) }
    }
    /// `sti; hlt` back to back: no interrupt can slip in between the two.
    #[inline(always)]
    pub fn enable_and_hlt() {
        unsafe { asm!("sti; hlt", options(nomem, nostack)) }
    }
    #[inline(always)]
    pub fn int3() {
        unsafe { asm!("int3", options(nomem, nostack)) }
    }
    pub fn are_enabled() -> bool {
        let flags: usize;
        #[cfg(target_arch = "x86_64")]
        unsafe { asm!("pushfq; pop {}", out(reg) flags, options(nomem, preserves_flags)) };
        #[cfg(target_arch = "x86")]
        unsafe { asm!("pushfd; pop {}", out(reg) flags, options(nomem, preserves_flags)) };
        flags & (1 << 9) != 0
    }
    /// Run `f` with interrupts off, restoring the previous state afterwards.
    pub fn without_interrupts<F: FnOnce() -> R, R>(f: F) -> R {
        let was = are_enabled();
        if was {
            disable();
        }
        let r = f();
        if was {
            enable();
        }
        r
    }
}

pub mod port {
    use core::arch::asm;
    use core::marker::PhantomData;

    pub trait PortValue: Copy {
        unsafe fn read_from(port: u16) -> Self;
        unsafe fn write_to(port: u16, v: Self);
    }
    impl PortValue for u8 {
        unsafe fn read_from(port: u16) -> u8 {
            let v: u8;
            asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack, preserves_flags));
            v
        }
        unsafe fn write_to(port: u16, v: u8) {
            asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack, preserves_flags));
        }
    }
    impl PortValue for u16 {
        unsafe fn read_from(port: u16) -> u16 {
            let v: u16;
            asm!("in ax, dx", out("ax") v, in("dx") port, options(nomem, nostack, preserves_flags));
            v
        }
        unsafe fn write_to(port: u16, v: u16) {
            asm!("out dx, ax", in("dx") port, in("ax") v, options(nomem, nostack, preserves_flags));
        }
    }

    pub struct Port<T: PortValue> {
        port: u16,
        _t: PhantomData<T>,
    }
    impl<T: PortValue> Port<T> {
        pub const fn new(port: u16) -> Self {
            Port { port, _t: PhantomData }
        }
        pub unsafe fn read(&mut self) -> T {
            T::read_from(self.port)
        }
        pub unsafe fn write(&mut self, v: T) {
            T::write_to(self.port, v)
        }
    }
}

/// Short name of the CPU architecture this kernel was built for.
#[cfg(target_arch = "x86_64")]
pub const ARCH: &str = "x86_64";
#[cfg(target_arch = "x86")]
pub const ARCH: &str = "i686";
