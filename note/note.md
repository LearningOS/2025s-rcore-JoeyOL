+ 进程
  
  为了在用户态就可以借助操作系统的服务动态灵活地管理和控制应用地执行，我们需要在已有的任务的抽象的基础上进一步扩展，形成新的抽象：**进程**，并实现若干基于 **进程**的强大系统调用，进程的功能包括：**创建；销毁；等待；信息；其他**

  ![alt text](image.png)

+ 进程概念
  
  进程就是操作系统选取某个可执行文件并对其进行一次动态执行的过程，相比可执行文件，他的动态性主要体现在：

  + 他是一个过程，从时间上看有开始也有结束
  + 在该过程中对于可执行文件给出的需求要想应对 **硬件/虚拟资源** 进行 **动态绑定和解绑**
  
  而在本章我们要实现的进程模型中，一共有三个状态：就绪态、运行态和等待态；且有基于页表的地址空间；可以被操作系统调度来分时占用CPU执行，可以动态创建和退出；可以通过系统调用获得操作系统的服务

+ fork 系统调用
  
  系统中同一时间存在的每个进程都被一个不同的 **进程标识符** (PID, Process Identifier)所标识，在内核初始化完毕之后会创建一个进程——即 **用户初始进程** (Initial Process)，它是目前在内核中以硬编码方式创建的唯一一个进程，其他所有进程都是通过与i个名为`fork`的系统调用创建的，进程A调用`fork`系统调用之后，内核会创建一个新进程B，这个进程B和调用`fork`的进程A在它们分别返回用户态时几乎处于相同的状态，**除了`fork`函数的返回值之外** 子进程的拿到的返回值是`0`，父进程的返回值是子进程的`pid`

    ```Rust
    /// 功能：当前进程 fork 出来一个子进程。
    /// 返回值：对于子进程返回 0，对于当前进程则返回子进程的 PID 。
    /// syscall ID：220
    pub fn sys_fork() -> isize;
    ```

  如何让在子进程和父进程返回不同的值？很简单，只需设置`a0`寄存器的值为`0`即可，子进程在创建后第一次被调度的时候，会从`trap_return`函数开始

+ waitpid 系统调用
  
  当一个进程通过`exit`系统调用退出之后，他所占用的资源并不能够立即回收，一种典型的做法是在退出时进程立即回收一部分资源，并将该进程标记为 **僵尸进程** 之后再由父进程回收掉它所占据的全部资源，这样这个进程才被彻底销毁，销毁进程也很简单，直接将 **子进程的PCB drop掉就可以自动回收所有资源**

    ```Rust
    /// 功能：当前进程等待一个子进程变为僵尸进程，回收其全部资源并收集其返回值。
    /// 参数：pid 表示要等待的子进程的进程 ID，如果为 -1 的话表示等待任意一个子进程；
    /// exit_code 表示保存子进程返回值的地址，如果这个地址为 0 的话表示不必保存。
    /// 返回值：如果要等待的子进程不存在则返回 -1；否则如果要等待的子进程均未结束则返回 -2；
    /// 否则返回结束的子进程的进程 ID。
    /// syscall ID：260
    pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize;
    ```

  如果一个子进程的父进程先于子进程结束了，那么子进程的回收工作会由初始进程来接管

+ exec 系统调用
  
  用于执行不同的可执行文件：

    ```Rust
    /// 功能：将当前进程的地址空间清空并加载一个特定的可执行文件，返回用户态后开始它的执行。
    /// 参数：path 给出了要加载的可执行文件的名字；
    /// 返回值：如果出错的话（如找不到名字相符的可执行文件）则返回 -1，否则不应该返回。
    /// syscall ID：221
    pub fn sys_exec(path: &str) -> isize;
    ```

  利用`fork`和`exec`的组合就可以在进程内`fork`出一个子进程并执行一个特定的可执行文件

+ 用户初始进程 initproc
  
    ```Rust
    // user/src/bin/initproc.rs

    #![no_std]
    #![no_main]

    #[macro_use]
    extern crate user_lib;

    use user_lib::{
        fork,
        wait,
        exec,
        yield_,
    };

    #[no_mangle]
    fn main() -> i32 {
        if fork() == 0 {
            exec("user_shell\0");
        } else {
            loop {
                let mut exit_code: i32 = 0;
                let pid = wait(&mut exit_code);
                if pid == -1 {
                    yield_();
                    continue;
                }
                println!(
                    "[initproc] Released a zombie process, pid={}, exit_code={}",
                    pid,
                    exit_code,
                );
            }
        }
        0
    }
    ```

  可以看出初始进程会在一开始创建一个子进程执行`user_shell`应用程序，并在之后循环回收子进程

    ```Rust
    // user/src/bin/user_shell.rs

    #![no_std]
    #![no_main]

    extern crate alloc;

    #[macro_use]
    extern crate user_lib;

    const LF: u8 = 0x0au8;
    const CR: u8 = 0x0du8;
    const DL: u8 = 0x7fu8;
    const BS: u8 = 0x08u8;

    use alloc::string::String;
    use user_lib::{fork, exec, waitpid, yield_};
    use user_lib::console::getchar;

    #[no_mangle]
    pub fn main() -> i32 {
        println!("Rust user shell");
        let mut line: String = String::new();
        print!(">> ");
        loop {
            let c = getchar();
            match c {
                LF | CR => {
                    println!("");
                    if !line.is_empty() {
                        line.push('\0');
                        let pid = fork();
                        if pid == 0 {
                            // child process
                            if exec(line.as_str()) == -1 {
                                println!("Error when executing!");
                                return -4;
                            }
                            unreachable!();
                        } else {
                            let mut exit_code: i32 = 0;
                            let exit_pid = waitpid(pid as usize, &mut exit_code);
                            assert_eq!(pid, exit_pid);
                            println!(
                                "Shell: Process {} exited with code {}",
                                pid, exit_code
                            );
                        }
                        line.clear();
                    }
                    print!(">> ");
                }
                BS | DL => {
                    if !line.is_empty() {
                        print!("{}", BS as char);
                        print!(" ");
                        print!("{}", BS as char);
                        line.pop();
                    }
                }
                _ => {
                    print!("{}", c as char);
                    line.push(c as char);
                }
            }
        }
    }
    ```

+ 进程标识符和内核栈
  
  之前我们用应用程序的加载顺序来标识一个进程，而现在有了进程的概念之后，同一时间存在的所有进程都有一个唯一的进程标识符，同样使用RAII的思想，将其抽象为一个`PidHandle`类型，当它的生命周期结束之后对应的整数会被编译器自动回收：

    ```Rust
    // os/src/task/pid.rs

    pub struct PidHandle(pub usize);
    // os/src/task/pid.rs

    struct PidAllocator {
        current: usize,
        recycled: Vec<usize>,
    }

    impl PidAllocator {
        pub fn new() -> Self {
            PidAllocator {
                current: 0,
                recycled: Vec::new(),
            }
        }
        pub fn alloc(&mut self) -> PidHandle {
            if let Some(pid) = self.recycled.pop() {
                PidHandle(pid)
            } else {
                self.current += 1;
                PidHandle(self.current - 1)
            }
        }
        pub fn dealloc(&mut self, pid: usize) {
            assert!(pid < self.current);
            assert!(
                self.recycled.iter().find(|ppid| **ppid == pid).is_none(),
                "pid {} has been deallocated!", pid
            );
            self.recycled.push(pid);
        }
    }

    lazy_static! {
        static ref PID_ALLOCATOR : UPSafeCell<PidAllocator> = unsafe {
            UPSafeCell::new(PidAllocator::new())
        };
    }
    ```
  
  同样的，我们还需要将内核栈中的应用编号替换为进程标识符

    ```Rust
    // os/src/task/pid.rs

    /// Return (bottom, top) of a kernel stack in kernel space.
    pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
        let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
        let bottom = top - KERNEL_STACK_SIZE;
        (bottom, top)
    }

    impl KernelStack {
        pub fn new(pid_handle: &PidHandle) -> Self {
            let pid = pid_handle.0;
            let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
            KERNEL_SPACE
                .exclusive_access()
                .insert_framed_area(
                    kernel_stack_bottom.into(),
                    kernel_stack_top.into(),
                    MapPermission::R | MapPermission::W,
                );
            KernelStack {
                pid: pid_handle.0,
            }
        }
        pub fn push_on_top<T>(&self, value: T) -> *mut T where
            T: Sized, {
            let kernel_stack_top = self.get_top();
            let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
            unsafe { *ptr_mut = value; }
            ptr_mut
        }
        pub fn get_top(&self) -> usize {
            let (_, kernel_stack_top) = kernel_stack_position(self.pid);
            kernel_stack_top
        }
    }

    // os/src/task/pid.rs

    impl Drop for KernelStack {
        fn drop(&mut self) {
            let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
            let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
            KERNEL_SPACE
                .exclusive_access()
                .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
        }
    }
    ```

  为`KernelStack`实现`Drop` Trait，一旦他的生命周期结束则在内核地址空间中会自动回收它所占用的空间

+ 进程控制块
  
  对任务控制块 `TaskControlBlock` 进行若干改动让他承担进程控制块的功能：

    ```Rust
    // os/src/task/task.rs

    pub struct TaskControlBlock {
        // immutable
        pub pid: PidHandle,
        pub kernel_stack: KernelStack,
        // mutable
        inner: UPSafeCell<TaskControlBlockInner>,
    }

    pub struct TaskControlBlockInner {
        pub trap_cx_ppn: PhysPageNum,
        pub base_size: usize,
        pub task_cx: TaskContext,
        pub task_status: TaskStatus,
        pub memory_set: MemorySet,
        pub parent: Option<Weak<TaskControlBlock>>,
        pub children: Vec<Arc<TaskControlBlock>>,
        pub exit_code: i32,
    }
    ```

  注意到在维护父子进程关系的时候大量用到了引用计数 `Arc/Weak` 进程控制块的本体是被放在内核堆上的，对于它的一切访问都是通过智能指针 `Arc/Weak` 来进行的，这样是为了便与建立父子进程的双向链接关系，当且仅当智能指针 `Arc` 的引用计数变为0的时候，进程控制块以及各类绑定到它上面的各类资源才会被回收

+ 任务管理器
  
  在前面的章节中，任务管理器 `TaskManager` 不仅负责管理所有的任务，还维护着CPU当前执行的任务，这种设计并不能扩展到后续的多核环境，因此我们需要将对于CPU的监控职能拆分到另一个处理器管理结构 `Processor` 中去，任务管理本身仅负责管理所有的任务

    ```Rust
    // os/src/task/manager.rs

    pub struct TaskManager {
        ready_queue: VecDeque<Arc<TaskControlBlock>>,
    }

    /// A simple FIFO scheduler.
    impl TaskManager {
        pub fn new() -> Self {
            Self { ready_queue: VecDeque::new(), }
        }
        pub fn add(&mut self, task: Arc<TaskControlBlock>) {
            self.ready_queue.push_back(task);
        }
        pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
            self.ready_queue.pop_front()
        }
    }

    lazy_static! {
        pub static ref TASK_MANAGER: UPSafeCell<TaskManager> = unsafe {
            UPSafeCell::new(TaskManager::new())
        };
    }

    pub fn add_task(task: Arc<TaskControlBlock>) {
        TASK_MANAGER.exclusive_access().add(task);
    }

    pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
        TASK_MANAGER.exclusive_access().fetch()
    }
    ```
  
  可以看出目前的调度算法是最简单的RR算法

+ 处理器管理结构
    
    ```Rust
    // os/src/task/processor.rs

    pub struct Processor {
        current: Option<Arc<TaskControlBlock>>,
        idle_task_cx: TaskContext,
    }

    impl Processor {
        pub fn new() -> Self {
            Self {
                current: None,
                idle_task_cx: TaskContext::zero_init(),
            }
        }
    }

    // os/src/task/processor.rs

    impl Processor {
        pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
            self.current.take()
        }
        pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
            self.current.as_ref().map(|task| Arc::clone(task))
        }
    }

    pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
        PROCESSOR.exclusive_access().take_current()
    }

    pub fn current_task() -> Option<Arc<TaskControlBlock>> {
        PROCESSOR.exclusive_access().current()
    }

    pub fn current_user_token() -> usize {
        let task = current_task().unwrap();
        let token = task.inner_exclusive_access().get_user_token();
        token
    }

    pub fn current_trap_cx() -> &'static mut TrapContext {
        current_task().unwrap().inner_exclusive_access().get_trap_cx()
    }
    ```

  `Processor` 有一个不同的idle控制流，它允许在这个CPU核的启动栈上，功能是尝试从任务管理器中取出一个任务来在当前CPU核上执行，在内核初始化完毕之后，就会调用 `run_tasks` 函数来进入idle控制流

    ```Rust
    // os/src/task/processor.rs

    pub fn run_tasks() {
        loop {
            let mut processor = PROCESSOR.exclusive_access();
            if let Some(task) = fetch_task() {
                let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
                // access coming task TCB exclusively
                let mut task_inner = task.inner_exclusive_access();
                let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
                task_inner.task_status = TaskStatus::Running;
                // stop exclusively accessing coming task TCB manually
                drop(task_inner);
                processor.current = Some(task);
                // stop exclusively accessing processor manually
                drop(processor);
                unsafe {
                    __switch(
                        idle_task_cx_ptr,
                        next_task_cx_ptr,
                    );
                }
            }
        }
    }

    impl Processor {
        fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
            &mut self.idle_task_cx as *mut _
        }
    }
    ```
  
  可以看出idle控制流就是一个调度器，那么如何切换到调度器进程呢？考虑到idle控制流一直在内核中运行，因此我们不需要切换页表，只需要切换到idle控制流的栈即可，具体实现通过调用 `schedule` 函数切换到idle控制流，开启新一轮的任务调度

    ```Rust
    // os/src/task/processor.rs

    pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
        let mut processor = PROCESSOR.exclusive_access();
        let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
        drop(processor);
        unsafe {
            __switch(
                switched_task_cx_ptr,
                idle_task_cx_ptr,
            );
        }
    }
    ```

+ 进程资源回收机制
  
  当应用调用 `sys_exit` 系统调用主动退出或者出错之后由内核终止之后，会在内核中调用 `exit_current_and_run_next` 函数退出当前进程并切换到下一个进程：

    ```Rust
    // os/src/mm/memory_set.rs

    impl MemorySet {
        pub fn recycle_data_pages(&mut self) {
            self.areas.clear();
        }
    }

    // os/src/task/mod.rs

    pub fn exit_current_and_run_next(exit_code: i32) {
        // take from Processor
        let task = take_current_task().unwrap();
        // **** access current TCB exclusively
        let mut inner = task.inner_exclusive_access();
        // Change status to Zombie
        inner.task_status = TaskStatus::Zombie;
        // Record exit code
        inner.exit_code = exit_code;
        // do not move to its parent but under initproc

        // ++++++ access initproc TCB exclusively
        {
            let mut initproc_inner = INITPROC.inner_exclusive_access();
            for child in inner.children.iter() {
                child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
                initproc_inner.children.push(child.clone());
            }
        }
        // ++++++ stop exclusively accessing parent PCB

        inner.children.clear();
        // deallocate user space
        inner.memory_set.recycle_data_pages();
        drop(inner);
        // **** stop exclusively accessing current PCB
        // drop task manually to maintain rc correctly
        drop(task);
        // we do not have to save task context
        let mut _unused = TaskContext::zero_init();
        schedule(&mut _unused as *mut _);
    }
    ```

  可以看出我们先取出当前进程控制块而非得到一份拷贝，这是为了正确维护进程控制块的引用计数，可以看出在这个函数里面我们只是将地址空间中的逻辑段列表清空，但用来存放页表的那些物理页帧此时还不会被回收（会由父进程最后回收）

+ 父进程回收子进程资源
  
  父进程通过 `sys_waitpid` 系统调用来回收子进程的资源并收集 `exit_code` 

    ```Rust
    // os/src/syscall/process.rs

    /// If there is not a child process whose pid is same as given, return -1.
    /// Else if there is a child process but it is still running, return -2.
    pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
        let task = current_task().unwrap();
        // find a child process

        // ---- access current TCB exclusively
        let mut inner = task.inner_exclusive_access();
        if inner.children
            .iter()
            .find(|p| {pid == -1 || pid as usize == p.getpid()})
            .is_none() {
            return -1;
            // ---- stop exclusively accessing current PCB
        }
        let pair = inner.children
            .iter()
            .enumerate()
            .find(|(_, p)| {
                // ++++ temporarily access child PCB exclusively
                p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
                // ++++ stop exclusively accessing child PCB
            });
        if let Some((idx, _)) = pair {
            let child = inner.children.remove(idx);
            // confirm that child will be deallocated after removing from children list
            assert_eq!(Arc::strong_count(&child), 1);
            let found_pid = child.getpid();
            // ++++ temporarily access child TCB exclusively
            let exit_code = child.inner_exclusive_access().exit_code;
            // ++++ stop exclusively accessing child PCB
            *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
            found_pid as isize
        } else {
            -2
        }
        // ---- stop exclusively accessing current PCB automatically
    }

    // user/src/lib.rs

    pub fn wait(exit_code: &mut i32) -> isize {
        loop {
            match sys_waitpid(-1, exit_code as *mut _) {
                -2 => { yield_(); }
                // -1 or a real pid
                exit_pid => return exit_pid,
            }
        }
    }
    ```