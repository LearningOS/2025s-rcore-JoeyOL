//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use super::BIG_STRIDE;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    /// ch5 Stride调度
    pub fn stride_schedule(&mut self) -> Option<Arc<TaskControlBlock>> {
        let mut min_stride: usize = usize::MAX;
        let mut ret = None;

        for task in self.ready_queue.iter() {
            if task.as_ref().inner_exclusive_access().stride < min_stride {
                ret = Some(task.clone());
                // mark pid for removal
                min_stride = task.as_ref().inner_exclusive_access().stride;
            }
        }

        if let Some(next_task) = ret.as_ref() {
            // access next task exclusively
            let mut inner = next_task.inner_exclusive_access();
            // update stride
            inner.stride += BIG_STRIDE / inner.priority;
            // mark pid for removal
            self.ready_queue.retain(|x| x.pid.0 != next_task.pid.0);
        };

        ret
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// Stride scheduling
pub fn stride_schedule() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::stride_schedule");
    TASK_MANAGER.exclusive_access().stride_schedule()
}
