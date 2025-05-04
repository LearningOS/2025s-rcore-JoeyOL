+ 引言
  
  到目前为止，对于单核处理器，在任意一个时刻只会有一个进程被操作系统调度，从而在处理器上执行，到目前为止的并发，仅仅是进程间的并发，而对于一个进程内部，还没有并发性的体现，这就是线程出现的起因：**提高一个进程内的并发性**

  补充：
  + 并行 (parallel) 是指两个或者多个事件在同一时刻发生
  + 并发 (concurrent) 是指两个或者多个事件在同一时间间隔内发生
  
+ 线程定义
  
  线程是进程的组成部分，进程可包含多个线程，线程由线程ID、执行状态、当前指令指针(PC)、寄存器集合和栈组成，线程是可以被操作系统或者用户态调度器独立调度(Scheduling)和分派(Dispatch)的基本单位，进程则是系统进行资源分配和调度的基本单位

+ 并发相关术语
  
  1. 共享资源：不同的线程/进程都能访问的变量或者数据结构
  2. 临界区：访问共享资源的一段代码
  3. 竞态条件：多个线程/进程都进入临界区，都试图更新共享的数据结构，导致产生了不期望的结果
  4. 不确定性：多个线程/进程在执行过程中出现了竞态条件导致执行结果不确定
  5. 原子性：一系列操作要么全部完成，要么一个都没执行，不会看到中间状态，在数据库领域具有原子性的一系列操作称作为事务
  6. 互斥：一种原子性操作，能保证同一时间只有一个线程进入临界区，从而避免出现竞态条件
  7. 同步；多个并发执行的进程/线程在一些关键点上需要互相等待，这种相互制约等待的过程称为同步
  8. 死锁（dead lock）：一个线程/进程集合里面的每个线程/进程都在等待只能由这个集合中的其他一个线程/进程（包括他自身）才能引发的事件，这种情况就是死锁。
  9. 饥饿（hungry）：指一个可运行的线程/进程尽管能继续执行，但由于操作系统的调度而被无限期地忽视，导致不能执行的情况。

+ 用户态的线程管理
  
  线程的运行需要一个执行环境，这个执行环境可以是操作系统内核，也可以是更简单的用户态的一个线程管理运行时库，如果是基于用户态的线程管理运行时库来实现对线程的支持，那我们需要实现一个线程管理器，进而对线程的管理、调度和执行方式进行一些限定，由于是在用户态进行线程的创建，那么操作系统并不需要感知到这种线程的存在，如果一个线程A想要运行，只有等待目前正在运行的线程B主动交出处理器的使用权，从而让 **线程管理运行时库** 有机会得到处理器的使用权，且线程管理运行时库通过调度选择了线程A，再完成线程B和线程A的上下文切换之后，线程A才能占用处理器并运行，

+ 线程模型和重要系统调用
  
  每个线程的生命周期都与程序中的一个函数的一次执行绑定，也就是说，线程从该函数入口点开始执行，当函数返回之后，线程也随之退出，因此再创建线程的时候我们需要提供程序中的一个函数让线程来执行这个函数，目前我们只实现一种非常简单的线程模型：

  + 线程有三种状态：就绪态、运行态和阻塞态
  + 同进程下的所有线程共享所属进程的地址空间和其他共享资源
  + 线程可以被操作系统调度来分时占用CPU执行
  + 线程可以动态创建和退出
  + 同进程下的多个线程不像进程一样存在父子关系，但是有一个特殊的主线程在他所属进程被创建的时候产生，应用程序的 `main` 函数就运行在这个主线程上，当主线程退出后，整个进程就会结束

+ 线程的创建和退出
  
  在创建一个新的线程时，内核会为每个线程分配一组专属于该线程的资源：用户栈、Trap上下文和内核栈，前两个在线程所属进程的地址空间中，内核栈在内核地址空间中，因此创建新的线程时 **无需建立新的地址空间** 

  在C/Rust语言中，当线程执行的函数返回之后，线程会自动退出，其实现原理是当函数返回之后，会自动跳转到用户态一段预先设置好的代码，在这段代码中通过系统调用实现线程退出的操作，线程退出之后，只会回收用户栈和Trap上下文，内核资源需要在进程内使用`waittid`系统调用来回收

  在实现线程之后的用户地址空间如下：

  ![alt text](image.png)

  可以看到每个线程都有自己独立的用户栈和Trap上下文，且由它们在所属进程的地址空间中的位置可以由TID计算得到，在低地址空间中，`ustack_base`按照TID从小到大的顺序向高地址放置线程的用户栈，在高地址空间中，最高的虚拟页依旧是作为跳板页，跳板页的下面向低地址按照TID从小到大的顺序放置线程的Trap上下文，因此只需要知道线程的TID，就可以计算得到线程的用户栈和Trap上下文的位置

  线程创建的代码实现如下：

  ```Rust
  // os/src/syscall/thread.rs

  pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
      let task = current_task().unwrap();
      let process = task.process.upgrade().unwrap();
      // create a new thread
      let new_task = Arc::new(TaskControlBlock::new(
          Arc::clone(&process),
          task.inner_exclusive_access().res.as_ref().unwrap().ustack_base,
          true,
      ));
      // add new task to scheduler
      add_task(Arc::clone(&new_task));
      let new_task_inner = new_task.inner_exclusive_access();
      let new_task_res = new_task_inner.res.as_ref().unwrap();
      let new_task_tid = new_task_res.tid;
      let mut process_inner = process.inner_exclusive_access();
      // add new thread to current process
      let tasks = &mut process_inner.tasks;
      while tasks.len() < new_task_tid + 1 {
          tasks.push(None);
      }
      tasks[new_task_tid] = Some(Arc::clone(&new_task));
      let new_task_trap_cx = new_task_inner.get_trap_cx();
      *new_task_trap_cx = TrapContext::app_init_context(
          entry,
          new_task_res.ustack_top(),
          kernel_token(),
          new_task.kstack.get_top(),
          trap_handler as usize,
      );
      (*new_task_trap_cx).x[10] = arg;
      new_task_tid as isize
  }
  ```

  线程退出的代码如下：

  ```Rust
  // os/src/task/mod.rs

  pub fn exit_current_and_run_next(exit_code: i32) {
      let task = take_current_task().unwrap();
      let mut task_inner = task.inner_exclusive_access();
      let process = task.process.upgrade().unwrap();
      let tid = task_inner.res.as_ref().unwrap().tid;
      // record exit code
      task_inner.exit_code = Some(exit_code);
      task_inner.res = None;
      // here we do not remove the thread since we are still using the kstack
      // it will be deallocated when sys_waittid is called
      drop(task_inner);
      drop(task);
      // however, if this is the main thread of current process
      // the process should terminate at once
      if tid == 0 {
          let pid = process.getpid();
          ...
          remove_from_pid2process(pid);
          let mut process_inner = process.inner_exclusive_access();
          // mark this process as a zombie process
          process_inner.is_zombie = true;
          // record exit code of main process
          process_inner.exit_code = exit_code;

          {
              // move all child processes under init process
              let mut initproc_inner = INITPROC.inner_exclusive_access();
              for child in process_inner.children.iter() {
                  child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
                  initproc_inner.children.push(child.clone());
              }
          }

          // deallocate user res (including tid/trap_cx/ustack) of all threads
          // it has to be done before we dealloc the whole memory_set
          // otherwise they will be deallocated twice
          let mut recycle_res = Vec::<TaskUserRes>::new();
          for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
              let task = task.as_ref().unwrap();
              // if other tasks are Ready in TaskManager or waiting for a timer to be
              // expired, we should remove them.
              //
              // Mention that we do not need to consider Mutex/Semaphore since they
              // are limited in a single process. Therefore, the blocked tasks are
              // removed when the PCB is deallocated.
              remove_inactive_task(Arc::clone(&task));
              let mut task_inner = task.inner_exclusive_access();
              if let Some(res) = task_inner.res.take() {
                  recycle_res.push(res);
              }
          }
          // dealloc_tid and dealloc_user_res require access to PCB inner, so we
          // need to collect those user res first, then release process_inner
          // for now to avoid deadlock/double borrow problem.
          drop(process_inner);
          recycle_res.clear();

          let mut process_inner = process.inner_exclusive_access();
          process_inner.children.clear();
          // deallocate other data in user space i.e. program code/data section
          process_inner.memory_set.recycle_data_pages();
          // drop file descriptors
          process_inner.fd_table.clear();
          // Remove all tasks except for the main thread itself.
          while process_inner.tasks.len() > 1 {
              process_inner.tasks.pop();
          }
      }
      drop(process);
      // we do not have to save task context
      let mut _unused = TaskContext::zero_init();
      schedule(&mut _unused as *mut _);
  }
  ```

+ 互斥锁的出现
  
  考虑这样一个多线程的例子：

  ```Rust
  // adder.rs

  static mut A: usize = 0;
  const THREAD_COUNT: usize = 4;
  const PER_THREAD: usize = 10000;
  fn main() {
      let mut v = Vec::new();
      for _ in 0..THREAD_COUNT {
          v.push(std::thread::spawn(|| {
              unsafe {
                  for _ in 0..PER_THREAD {
                      A = A + 1;
                  }
              }
          }));
      }
      for handle in v {
          handle.join().unwrap();
      }
      println!("{}", unsafe { A });
  }
  ```

  考察其中两个线程的指令执行过程：

  ![alt text](image-1.png)

  我们会发现操作系统的调度可能使得两个线程上的操作块交错出现，也就是两个操作块从开始到结束的时间区间存在交集，一旦出现这种情况，便会导致结果出现偏差，最终的结果取决于这种交错的情况出现多少次，因此多线程对共享资源的访问天然需求某种互斥性，当一个线程在进行操作的时候，共享资源处在不合法的中间状态，如果此时其他线程开始操作就会产生未定义行为，因此只有当操作完成，共享资源重新回到合法状态之后，之前操作的线程或者其他线程才能开始下一次操作，用形式化的语言表述如下：

  共享资源 (Shared Resources) 是指多个线程均能够访问的资源。线程对于共享资源进行操作的那部分代码被称为 临界区 (Critical Section)。在多线程并发访问某种共享资源的时候，为了正确性，必须要满足 互斥 (Mutual Exclusion) 访问要求，即同一时间最多只能有一个线程在这种共享资源的临界区之内。这样才能保证当一个线程开始操作时，共享资源总是处于合法状态，这保证了操作是有意义的

+ 原子指令
  
  在上述多线程例子中，由于计算简单，不借助锁机制也可以解决问题，因为其中的共享资源是一个64位的无符号整型，是一个非常简单的类型，对于这种原生类型，现代指令集架构额外提供一组 **原子指令** 在某些架构上只需要一条原子指令就能够完成访存、算术运算等一系列操作，即将临界区缩小为一条原子指令：

  ```Rust
  // adder_fixed.rs

  use std::sync::atomic::{AtomicUsize, Ordering};
  static A: AtomicUsize = AtomicUsize::new(0);
  const THREAD_COUNT: usize = 4;
  const PER_THREAD: usize = 10000;
  fn main() {
      let mut v = Vec::new();
      for _ in 0..THREAD_COUNT {
          v.push(std::thread::spawn(|| {
              for _ in 0..PER_THREAD {
                  A.fetch_add(1, Ordering::Relaxed);
              }
          }));
      }
      for handle in v {
          handle.join().unwrap();
      }
      println!("{}", A.load(Ordering::Relaxed));
  }
  ```

  Rust 核心库在 core::sync::atomic 中提供了很多原子类型，比如我们这里可以使用 usize 对应的原子类型 AtomicUsize ，它支持很多原子操作。比如，第 12 行 fetch_add 的功能是将 A 的值加一并返回 A 之前的值，这其中涉及到读取内存、算术运算和写回内存，但是却只需要这一个操作就能同时完成。这种原子操作基于硬件提供的原子指令，硬件可以保证其 原子性 (Atomicity)，含义是该操作的一系列功能要么全部完成，要么都不完成，而不会出现有些完成有些未完成的情况。原子性中的“原子”是为了强调操作中的各种功能作为一个整体不可分割的属性。这种由硬件提供的 原子指令是整个计算机系统中最根本的原子性和互斥性的来源 。无论软件执行了哪些指令，也无论 CPU 执行指令的时候出现了哪些中断/异常，又或者多个 CPU 同时访问内存中同一个位置这种情形，都不能破坏原子指令的原子性

+ 锁的形态和功能
  
  锁是附加在一种共享资源上的一种标记，最简单的情况下它只需要由两种状态：上锁和空闲，上锁状态表示此时有某个线程可以进入临界区，线程在成功进入临界区之后锁也需要从空闲转换为上锁状态，锁的两个基本操作是上锁和解锁，在线程进入临界区之前和退出临界区之后分别需要成功上锁和解锁，通过这种方式我们就可以保证临界区的互斥性

  如果使用锁，那么上述程序应当改成如下结构：

  ```Rust
  // adder_mutex1.rs

  use std::sync::Mutex;
  static mut A: usize = 0;
  static LOCK: Mutex<bool> = Mutex::new(true);
  const THREAD_COUNT: usize = 4;
  const PER_THREAD: usize = 10000;
  fn main() {
      let mut v = Vec::new();
      for _ in 0..THREAD_COUNT {
          v.push(std::thread::spawn(|| {
              for _ in 0..PER_THREAD {
                  let _lock = LOCK.lock();
                  unsafe { A = A + 1; }
              }
          }));
      }
      for handle in v {
          handle.join().unwrap();
      }
      println!("{}", unsafe { A });
  }
  ```

+ 锁的评价指标
  
  锁机制对应有多种不同的实现，对于一种实现而言，我们常常用以下的指标来评估这种实现能够达成锁的功能：

  + 忙则等待
  + 空闲进入
  + 有界等待
  + 让权等待（可选）

+ 锁的纯用户态软件实现
  
  我们知道锁的本质是一个标记，表明目前是否已经有线程进入共享资源的临界区了，于是最简单的实现思路就是加入一个新的全局变量用作这个标记：

  ```Rust
  // user/src/bin/adder_simple_spin.rs

  static mut OCCUPIED: bool = false;

  unsafe fn lock() {
      while vload!(OCCUPIED) {}
      OCCUPIED = true;
  }

  unsafe fn unlock() {
      OCCUPIED = false;
  }
  ```

  第 6 行不断 while 循环直到标记被改为 false ，在循环体内则不做任何事情，这是一种典型的 忙等待 (Busy Waiting) 策略，它也被形象地称为 自旋 (Spinning)，我们目前基于单核 CPU ，如果循环第一次迭代发现标记为 true 的话，在触发时钟中断切换到其他线程之前，无论多少次查看标记都必定为 true ，因为当前线程不会修改标记。这就会造成 CPU 资源的严重浪费。针对这种场景， Rust 提供了 spin_loop_hint 函数，我们可以在循环体内调用该函数来通知 CPU 当前线程正处于忙等待状态，于是 CPU 可能会进行一些优化（比如降频减少功耗等），其在不同平台上有不同表现

  但是它能够保证最关键的互斥访问吗？很可惜并不能，检查 `lock` 的汇编代码大致分为三个阶段，每个阶段由一条或者多条指令组成：

  1. 将标记的值加载到寄存器 reg
  2. 条件跳转，如果 reg 为 1 则跳转回第一阶段开始新一轮循环，否则就向下进行
  3. 将标记赋值为 1
   
  我们很容易构造出一种时间片分割的方式使得互斥访问失效：假设某时刻标记 OCCUPIED 的值为 false ，线程 T0 和 T1 都准备进入临界区。假设先切换到 T0 ，它经历 1、2 阶段，看到标记为 false ，认为自己能够进入临界区，但是在执行关键的 3 阶段之前被操作系统切换到线程 T1 。T1 也经历 1、2阶段，由于 T0 并没有修改标记，它也认为自己能够进入临界区。接下来显然线程 T0 和 T1 能够同时进入临界区了，这就违背了互斥访问要求

  问题的本质是：在这个实现中，标记 OCCUPIED 也成为了多线程均可访问的 共享资源 ，那么 它也需求互斥访问 。而我们并没有吸取 adder.rs 的教训，我们让操作为多阶段多指令的 OCCUPIED 无任何保护的暴露在操作系统调度面前，那么自然也会发生和 adder.rs 类似的问题

+ 多标记组合
  
  既然仅使用一个标记不行，那么尝试使用多标记组合来表示锁的状态，比如下面的Peterson算法就适合两个线程之间的互斥访问：

  ```Rust
  // user/src/bin/adder_peterson_spin.rs

  /// FLAG[i]=true 表示线程 i 想要进入或已经进入临界区
  static mut FLAG: [bool; 2] = [false; 2];
  /// TURN=i 表示轮到线程 i 进入临界区
  static mut TURN: usize = 0;

  /// id 表示当前的线程 ID ，为 0 或 1
  unsafe fn lock(id: usize) {
      FLAG[id] = true;
      let j = 1 - id;
      TURN = j;
      // Tell the compiler not to reorder memory operations
      // across this fence.
      compiler_fence(Ordering::SeqCst);
      // Why do we need to use volatile_read here?
      // Otherwise the compiler will assume that they will never
      // be changed on this thread. Thus, they will be accessed
      // only once!
      while vload!(FLAG[j]) && vload!(TURN) == j {}
      // while FLAG[j] && TURN == j {}
  }

  unsafe fn unlock(id: usize) {
      FLAG[id] = false;
  }
  ```

  考虑上述实现是否满足锁的基本指标：

  1. 互斥访问：
    
    + 考虑两个线程的 `lock` 操作不重叠的情况，这种情况比较简单，假设$T_j$已经成功进入了临界区且尚未退出，此时$T_i$尝试进入临界区，那么会因为此时 $\text{flag}_j$ 一定为 true 导致 $T_i$ 陷入忙等
    + 第二种情况考虑两个线程的 `lock` 操作由于操作系统调度出现了交错现象，也即两个线程在同段时间内尝试进入临界区，那么此时会有 $\text{flag}_i = \text{flag}_j = \text{true}$，那么忙等的条件就可以化简为 turn 的取值，在 **单核CPU** 上，线程$T_i$和线程$T_j$对于 turn 的修改总会有一个在时间上靠后，因此最后不论是哪一个线程最后修改了 turn，此时另一个线程最终一定能够进入临界区

  2. 空闲则入：
   
     对于空闲则入，如果只有单个线程要进入临界区，那么它一定能进去。如果两个线程要同时进入的话，只需要等到两个线程对于 turn 的修改均完成就能够确定哪个线程进入临界区，这一定能够在有限时间内做到
  
  3. 有界等待：
     
     假设线程$T_i$没有竞争过$T_j$而在 `lock` 中陷入忙等，$T_j$成功进入了临界区，在$T_j$离开临界区之后，由于此时 turn 只会被修改为 i，那么最终一定会使$T_j$不可能在$T_i$进入临界区之前再次进入临界区了

+ 关闭中断
  
  