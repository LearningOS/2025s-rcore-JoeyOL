# 编程作业
1. 首先维护了之前的系统调用`sys_get_time` `sys_mmap` `sys_munmap`，使其能够适应本章节新的进程结构
2. 实现新的进程创建系统调用 `spawn`，实现了除了 `fork + exec` 之外的创建新进程的方法，并且能够正确维护进程之间的父子关系，且与 `fork + exec` 不同，子进程不需要复制父进程的地址空间，然后再清空创建新的地址空间，成功执行时会放回子进程pid，否则返回-1（比如传入无效的文件名）
3. 实现stride调度算法，在进程控制块中维护新的字段 `stride` 和 `priority` 每次调度时会选择 `stride` 最小的进程执行，并更新 `stride = BigStride / priority`，实现之后，每个进程分配的时间与其优先级成正比，并添加了新的系统调用 `sys_set_priority` 能够更改进程的 `priority`

# 问答作业
1. 当两个进程的`pass = 10`，且初始`p1.stride = 255`，`p2.stride = 250`时：
    + p2 执行后​​：`p2.stride`增加 10 变为 260，由于 8 位无符号数溢出，结果为 260 - 256 = 4
    + ​比较`p1.stride=255`和`p2.stride=4`​​：若直接按无符号数比较，4 < 255，应选 p2

    因此实际情况会选择 p2 执行
2. 因为优先级$\ge$ 2因此步长$pass = BigStride / priority ≤ BigStride/2$，而每次调度时会选择 stride 最小的进程，其 stride 增加 pass，初始插值均为0，因此每次调度之后差值最多增加 $PassMax = BigStride/2$，当因此所有进程优先级$\ge$ 2 时，$STRIDE\_MAX−STRIDE\_MIN≤BigStride/2$
3. 代码是实现如下：
   
   ```Rust
   use core::cmp::Ordering;

    const BIG_STRIDE: u64 = 255; // 8-bit stride

    struct Stride(u64);

    impl PartialOrd for Stride {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            // 计算环形差值（模 BIG_STRIDE + 1）
            let diff = self.0.wrapping_sub(other.0) & BIG_STRIDE;
            // 差值超过半周则反转比较结果
            if diff <= BIG_STRIDE / 2 {
                Some(Ordering::Greater) // self > other
            } else {
                Some(Ordering::Less)     // self < other
            }
        }
    }

    impl PartialEq for Stride {
        fn eq(&self, _: &Self) -> bool {
            false // 假设永不相等
        }
    }
   ```

# 荣誉准则

在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：**无**

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：**https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter3/**

1. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

2. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。