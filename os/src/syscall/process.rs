//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, get_syscall_counter, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
// 这个系统调用有三种功能，根据 trace_request 的值不同，执行不同的操作：
//如果 trace_request 为 0，则 id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值。此时应忽略 data 参数。返回值为 id 地址处的值。
//如果 trace_request 为 1，则 id 应被视作 *mut u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
//如果 trace_request 为 2，表示查询当前任务调用编号为 id 的系统调用的次数，返回值为这个调用次数。本次调用也计入统计 。
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            // read a byte from the address pointed by id
            let id = _id as *const u8;
            let value = unsafe { *id };
            return value as isize;
        }
        1 => {
            // write a byte to the address pointed by id
            let id = _id as *mut u8;
            let data = _data as u8;
            unsafe {
                *id = data;
            }
            return 0;
        }
        2 => {
            // get syscall counter
            let syscall_id = _id;
            let count = get_syscall_counter(syscall_id);
            // panic!("task:{}, syscall_id: {}, count: {}, map_id:{}",get_current_task(), syscall_id, count, get_syscall_map(syscall_id));
            return count as isize;
        }
        _ => {
            -1
        }
    }
}
