//! Process management syscalls
use core::usize;

use crate::config::PAGE_SIZE;
use crate::
    task::{ exit_current_and_run_next, get_syscall_counter, suspend_current_and_run_next, mmap, munmap}
;
use crate::task::change_program_brk;

use crate::mm::{MapPermission, VirtAddr, PhysAddr};
use crate::task::current_user_token;
use crate::mm::PageTable;
use crate::timer::get_time_ms;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    if let Some(entry) = PageTable::from_token(current_user_token())
        .translate(VirtAddr(_ts as usize).floor()) 
    {
        if entry.user() && entry.writable() {
            let ppn: usize = PhysAddr::from(entry.ppn()).0 + VirtAddr(_ts as usize).page_offset();
            unsafe {
                let ts = &mut *(ppn as *mut TimeVal);
                ts.sec = get_time_ms() / 1000;
                ts.usec = (get_time_ms() % 1000) * 1000;
            }
        }
    }
    0
}

// TODO: implement the syscall
// 这个系统调用有三种功能，根据 trace_request 的值不同，执行不同的操作：
//如果 trace_request 为 0，则 id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值。此时应忽略 data 参数。返回值为 id 地址处的值。
//如果 trace_request 为 1，则 id 应被视作 *mut u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
//如果 trace_request 为 2，表示查询当前任务调用编号为 id 的系统调用的次数，返回值为这个调用次数。本次调用也计入统计 。
/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            // Read a byte from the address pointed by id
            if let Some(entry) = PageTable::from_token(current_user_token())
                .translate(VirtAddr(_id).floor())
            {
                if entry.user() && entry.readable() {
                    let ppn:usize = PhysAddr::from(entry.ppn()).0 + VirtAddr(_id).page_offset();
                    unsafe { return *(ppn as *const u8) as isize; }
                }
            }
            -1
        }
        1 => {
            // Write a byte to the address pointed by id
            if let Some(entry) = PageTable::from_token(current_user_token())
                .translate(VirtAddr(_id).floor())
            {
                if entry.user() && entry.writable() {
                    let ppn: usize= PhysAddr::from(entry.ppn()).0 + VirtAddr(_id).page_offset();
                    unsafe { *(ppn as *mut u8) = _data as u8; }
                    return 0;
                }
            }
            -1
        }
        2 => {
            // Get syscall counter
            get_syscall_counter(_id) as isize
        }
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if _start % PAGE_SIZE != 0 
        || (_port & !0x7 !=0) 
        || (_port & 0x7 == 0) {
        return -1;
    }
    else {
        let start = _start;
        let mut len = _len;
        if len % PAGE_SIZE != 0 {
            len += PAGE_SIZE - (len % PAGE_SIZE);
        }
        let mut map_permission = MapPermission::U;
        if _port & 0x1 != 0 {
            map_permission |= MapPermission::R;
        }
        if _port & 0x2 != 0 {
            map_permission |= MapPermission::W;
        }
        if _port & 0x4 != 0 {
            map_permission |= MapPermission::X;
        }
        let result = mmap(start, len, map_permission);
        return result;
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    else {
        let start = _start;
        let mut len = _len;
        if len % PAGE_SIZE != 0 {
            len += PAGE_SIZE - (len % PAGE_SIZE);
        }
        let result = munmap(start, len);
        return result;
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
