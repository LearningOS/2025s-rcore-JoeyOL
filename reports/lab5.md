+ 编程作业
  
  1. 兼容 `sys_get_time` 系统调用，使其能够通过 `sleep` 和 `sleep1` 测试，能够正确获取时间信息
  2. 实现 `enable_deadlock_detect` 系统调用，在进程控制块中维护一个是否启用死锁检测的字段，调用 `enable_deadlock_detect` 可以更改该字段，进而达到为当前进程启用或禁用死锁检测的功能
  3. 实现死锁检测算法，为每个线程维护其在所属进程中占用的锁的数量（对应allocation矩阵），以及需要的尚未分配的锁的数量（对应need矩阵），并在进程中维护各类资源的可以利用数量（对应available矩阵），最后在 `mutex_lock` 和 `semaphore_down` 加入死锁检测的代码，一旦检测到当前进程开启了死锁检测的功能之后，便会在每次调用 `mutex_lock` 和 `semaphore_down` 时执行这段代码，如果检测到死锁，应拒绝相应操作并返回 `-0xDEAD`

+ 问答作业
  
  1. + 回收资源包括：​

       1. ​用户态资源​​：
           
           ​用户栈（UStack）​​：每个线程独立分配的栈空间。​​线程 ID（TID）​​：通过 dealloc_tid 回收 TID 标识符

           Trap 上下文（Trap Context）​​：线程陷入内核时的上下文信息（如寄存器快照）
       
       2. 进程级资源​​：
           
           ​内存页（Memory Pages）​​：进程的代码段、数据段、堆栈等用户空间内存（通过 memory_set.recycle_data_pages 回收）
       
           ​文件描述符（File Descriptors）​​：进程打开的文件、管道、Socket 等（通过 fd_table.clear 关闭并释放）。

           ​子进程关系​​：将子进程转移给 init 进程，防止孤儿进程，​
       
           ​线程控制块（TaskControlBlock, TCB）​​：所有子线程的 TCB 内存（通过 process_inner.tasks.pop 移除并释放引用）

     + 其他线程的 `TaskControlBlock` 引用位置及回收必要性：

        1. ​​进程的任务列表包含每个线程的 `TCB`，主线程退出时，需通过 `process_inner.tasks.pop` 移除所有子线程的 `TCB`，确保这些线程的资源被回收。若未移除，`TCB` 的 `Arc` 引用计数无法归零，导致内存泄漏，因此必须回收
        2. 任务管理器（TaskManager）​包含处于就绪队列或定时器等待队列的线程，必须通过 `remove_inactive_task` 将 `TCB` 从任务管理器中移除回收这些线程 `TCB` 的引用，否则这些线程可能被错误调度（即使进程已终止），引发未定义行为或资源竞争
        3. ​同步原语（Mutex/Semaphore）的等待队列​也包含对 `TCB` 的引用，其会被隐式回收，同步因为原语的生命周期与进程绑定，当进程退出时，同步原语会被销毁，其等待队列中的 `TCB` 引用自动解除，无需显式操作