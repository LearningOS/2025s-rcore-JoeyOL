
use core::arch::asm;

fn sbi_call(which: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let ret: usize;
    unsafe {
        asm!(
            "ecall",
            in("a7") which,
            in("a0") arg1,
            in("a1") arg2,
            in("a2") arg3,
            lateout("a0") ret,
        );
    }
    ret
}

const SBI_SHUTDOWN: usize = 8;

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    panic!("It should shutdown");
}