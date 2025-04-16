#![no_std]
#![no_main]

use core::fmt::{self, Write};
mod lang_items;
mod sbi;
use sbi::shutdown;
// 此时为一个空程序，输出的汇编代码不会有任意信息，因为我们缺少一个程序入口函数 _start
// fn main() {
//     // println!("Hello, world!");
// }

// 构建用户态最小化执行环境

// 这一段代码给Rust编译器提供了入口函数 _start()
// 反汇编指令输出两条指令是一个死循环
// #[no_mangle]
// extern "C" fn _start() {
//     loop{};
// }

// 这一段看起来是一个合法的执行程序，但是执行它，会引发问题（输出段错误）
// 这是因为目前的执行环境还缺乏一个退出机制，即我们需要操作系统提供的 exit 系统调用来退出程序

// 嵌入汇编代码
core::arch::global_asm!(include_str!("entry.asm"));

// 清空栈数据
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize)
        .for_each(|addr| unsafe { *(addr as *mut u8) = 0 });
}

#[no_mangle]
extern "C" fn rust_main() {
    // 至此构建完成了第一个hello world程序
    // println!("Hello World!");
    // sys_exit(9);
    clear_bss();
    shutdown();
}

// 实现syscall
const  SYSCALL_EXIT: usize = 93;

// 这段代码封装了一个通用的 syscall 函数，允许用户通过传递系统调用号 id ，和参数 args 来发起系统调用
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret;
    unsafe {
        // Rust 宏，把它当成黑盒就好！
        core::arch::asm!(
            "ecall",
            // 在asm!宏中(内联汇编)，in的作用是将表达式的值加载到寄存器中，inlateout可以指定一个寄存器既做输入也作为输出
            inlateout("x10") args[0] => ret,
            in ("x11") args[1],
            in ("x12") args[2],
            in ("x17") id,
        );
    }
    ret
}

pub fn sys_exit(xstate: i32) -> isize {
    syscall(SYSCALL_EXIT, [xstate as usize, 0, 0])
}

// 实现一个字串显示

// 首先封装对 SYSCALL_WRITE 的系统调用

const SYSCALL_WRITE: usize = 64;

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    // sys_write的参数为文件描述符，字符串开始指针，长度
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

struct Stdout;

impl Write for Stdout {
    // 不支持格式化输出，如 format_args!("Hello {}!", "World")
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        sys_write(1, s.as_bytes());
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

// 实现Rust语言格式化宏

#[macro_export]
macro_rules! print {
    // $(, $($arg: tt)+)?，？表示参数可选，(, +)表示可以接受零个或者多个额外的参数
    ($fmt: literal $(, $($arg: tt)+)?) => {
        // 即上面实现的print函数。$crate::console::print的作用是为了防止命名冲突，使得外部模块也能正常调用这个宏
        $crate::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    // $(, $($arg: tt)+)?，？表示参数可选，(, +)表示可以接受零个或者多个额外的参数
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

