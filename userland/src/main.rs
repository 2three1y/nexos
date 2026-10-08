// NexOS userland — placeholder.
//
// User-space programs (shell, utilities, ELF-loaded apps) will live here.
// Unlike the kernel, userland links against the kernel's syscall ABI and runs
// in ring 3. For now this is a minimal placeholder so the workspace has a
// clean kernel/userland boundary from day one.

pub fn placeholder() -> u32 {
    0
}

fn main() {
    placeholder();
}