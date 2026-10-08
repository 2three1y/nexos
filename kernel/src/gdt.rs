//! Global Descriptor Table + Task State Segment.
//!
//! Kernel and user (ring 3) segments, plus a TSS that provides:
//!  - IST[0]: a separate stack for the double-fault handler, so even a kernel
//!    stack overflow is reported instead of triple-faulting;
//!  - RSP0: the stack the CPU switches to when an interrupt or syscall
//!    arrives while a ring-3 program is running.

use spin::Once;
use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST: u16 = 0;
const STACK_SIZE: usize = 4096 * 5;

#[repr(align(16))]
#[allow(dead_code)]
struct Stack([u8; STACK_SIZE]);
static mut DF_STACK: Stack = Stack([0; STACK_SIZE]);
static mut RSP0_STACK: Stack = Stack([0; STACK_SIZE]);

pub struct Selectors {
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub user_data: SegmentSelector,
    tss: SegmentSelector,
}

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<(GlobalDescriptorTable, Selectors)> = Once::new();

pub fn init() {
    let tss = TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST as usize] =
            VirtAddr::from_ptr(&raw const DF_STACK) + STACK_SIZE as u64;
        tss.privilege_stack_table[0] = VirtAddr::from_ptr(&raw const RSP0_STACK) + STACK_SIZE as u64;
        tss
    });
    let (gdt, sel) = GDT.call_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data = gdt.append(Descriptor::kernel_data_segment());
        let user_data = gdt.append(Descriptor::user_data_segment());
        let user_code = gdt.append(Descriptor::user_code_segment());
        let tss_sel = gdt.append(Descriptor::tss_segment(tss));
        (gdt, Selectors { kernel_code, kernel_data, user_code, user_data, tss: tss_sel })
    });
    gdt.load();
    unsafe {
        CS::set_reg(sel.kernel_code);
        SS::set_reg(sel.kernel_data);
        DS::set_reg(sel.kernel_data);
        ES::set_reg(sel.kernel_data);
        load_tss(sel.tss);
    }
}

pub fn selectors() -> &'static Selectors {
    &GDT.get().expect("GDT not initialised").1
}
