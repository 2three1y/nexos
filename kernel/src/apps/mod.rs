//! Built-in Looscid OS apps that run inside the kernel. Ring-3 apps live in
//! userland/. Every app is described by a manifest in kernel/catalog/ and
//! registered by its entry name in user.rs (see docs/APPS.md).

pub mod calc;
pub mod clock;
pub mod insomnia;
pub mod notes;
pub mod piano;
pub mod soundscape;
pub mod store;
pub mod sysinfo;
pub mod thoughts;
pub mod ui;
