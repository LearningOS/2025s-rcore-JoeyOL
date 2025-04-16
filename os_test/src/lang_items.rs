use core::panic::PanicInfo;

// 编写panic函数，通过标记 panic_handler 告知编译器采用我们的实现
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // 目前只是原地 loop
    loop {}
}