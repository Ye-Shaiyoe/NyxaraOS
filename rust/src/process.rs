use crate::println;
use crate::syscall::Registers;

const MAX_TASKS: usize = 8;
const STACK_SIZE: usize = 4096;
const QUANTUM_TICKS: u32 = 5;
const USER_CODE_ADDR: usize = 0x04000000;
const USER_STACK_ADDR: usize = 0x04001000;
const USER_CODE_SELECTOR: u32 = 0x1B;
const USER_DATA_SELECTOR: u32 = 0x23;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
}

#[derive(Copy, Clone)]
struct Task {
    pid: u32,
    state: TaskState,
    frame: *mut Registers,
    stack_top: u32,
    wake_at: u64,
    ticks: u64,
    name: &'static str,
}

impl Task {
    const fn empty() -> Self {
        Self {
            pid: 0,
            state: TaskState::Empty,
            frame: core::ptr::null_mut(),
            stack_top: 0,
            wake_at: 0,
            ticks: 0,
            name: "-",
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C, align(16))]
struct TaskStack([u8; STACK_SIZE]);

static mut TASKS: [Task; MAX_TASKS] = [Task::empty(); MAX_TASKS];
static mut STACKS: [TaskStack; MAX_TASKS] = [TaskStack([0; STACK_SIZE]); MAX_TASKS];
static mut CURRENT: usize = 0;
static mut NEXT_PID: u32 = 1;
static mut QUANTUM: u32 = QUANTUM_TICKS;
static mut TICKS: u64 = 0;
static mut INITIALIZED: bool = false;

extern "C" {
    fn irq_set_switch_frame(regs: *mut Registers, stack_top: u32);
    fn hlt();
    fn sti();
    static user_demo_start: u8;
    static user_demo_end: u8;
}

pub fn init() {
    unsafe {
        TASKS[0] = Task {
            pid: NEXT_PID,
            state: TaskState::Running,
            frame: core::ptr::null_mut(),
            stack_top: 0,
            wake_at: 0,
            ticks: 0,
            name: "shell",
        };
        NEXT_PID += 1;
        INITIALIZED = true;
    }

    crate::logln!(
        "[Scheduler] Round-robin scheduler initialized ({} tasks).",
        task_count()
    );
}

#[allow(dead_code)]
unsafe fn spawn(entry: extern "C" fn() -> !, name: &'static str) -> u32 {
    let slot = match TASKS.iter().position(|task| task.state == TaskState::Empty) {
        Some(slot) => slot,
        None => return 0,
    };

    let stack_top = STACKS[slot].0.as_ptr().add(STACK_SIZE) as usize;
    let frame =
        (stack_top & !0xF).wrapping_sub(core::mem::size_of::<Registers>()) as *mut Registers;
    core::ptr::write_bytes(frame as *mut u8, 0, core::mem::size_of::<Registers>());

    (*frame).ds = 0x10;
    (*frame).eip = entry as usize as u32;
    (*frame).cs = 0x08;
    (*frame).eflags = 0x202;
    (*frame).esp = stack_top as u32;

    let pid = NEXT_PID;
    NEXT_PID += 1;
    TASKS[slot] = Task {
        pid,
        state: TaskState::Ready,
        frame,
        stack_top: stack_top as u32,
        wake_at: 0,
        ticks: 0,
        name,
    };
    pid
}

#[allow(dead_code)]
unsafe fn spawn_user_demo() -> u32 {
    let code_size = (&user_demo_end as *const u8 as usize)
        .saturating_sub(&user_demo_start as *const u8 as usize);
    if code_size == 0 || code_size > crate::vmm::PAGE_SIZE {
        return 0;
    }

    let code_frame = match crate::pmm::alloc_frame() {
        Some(frame) => frame,
        None => return 0,
    };
    let stack_frame = match crate::pmm::alloc_frame() {
        Some(frame) => frame,
        None => return 0,
    };

    core::ptr::copy_nonoverlapping(
        &user_demo_start as *const u8,
        code_frame as *mut u8,
        code_size,
    );
    core::ptr::write_bytes(stack_frame as *mut u8, 0, crate::vmm::PAGE_SIZE);

    if crate::vmm::map_page(USER_CODE_ADDR, code_frame, crate::vmm::PAGE_USER).is_err()
        || crate::vmm::map_page(
            USER_STACK_ADDR,
            stack_frame,
            crate::vmm::PAGE_USER | crate::vmm::PAGE_WRITABLE,
        )
        .is_err()
    {
        return 0;
    }

    let slot = match TASKS.iter().position(|task| task.state == TaskState::Empty) {
        Some(slot) => slot,
        None => return 0,
    };
    let kernel_stack_top = STACKS[slot].0.as_ptr().add(STACK_SIZE) as usize;
    let frame = (kernel_stack_top & !0xF).wrapping_sub(core::mem::size_of::<Registers>())
        as *mut Registers;
    core::ptr::write_bytes(frame as *mut u8, 0, core::mem::size_of::<Registers>());

    (*frame).ds = USER_DATA_SELECTOR;
    (*frame).eip = USER_CODE_ADDR as u32;
    (*frame).cs = USER_CODE_SELECTOR;
    (*frame).eflags = 0x202;
    (*frame).useresp = (USER_STACK_ADDR + crate::vmm::PAGE_SIZE - 16) as u32;
    (*frame).ss = USER_DATA_SELECTOR;

    let pid = NEXT_PID;
    NEXT_PID += 1;
    TASKS[slot] = Task {
        pid,
        state: TaskState::Ready,
        frame,
        stack_top: kernel_stack_top as u32,
        wake_at: 0,
        ticks: 0,
        name: "ring3-demo",
    };
    crate::logln!("[Process] Ring 3 demo task created (pid={}).", pid);
    pid
}

#[no_mangle]
pub extern "C" fn nyxara_scheduler_tick(regs: *mut Registers) {
    unsafe {
        if !INITIALIZED {
            return;
        }

        TICKS += 1;
        TASKS[CURRENT].frame = regs;
        TASKS[CURRENT].stack_top = 0;
        TASKS[CURRENT].ticks += 1;

        for task in TASKS.iter_mut() {
            if task.state == TaskState::Blocked && task.wake_at <= TICKS {
                task.state = TaskState::Ready;
            }
        }

        if QUANTUM > 0 {
            QUANTUM -= 1;
        }
        if QUANTUM != 0 {
            return;
        }

        let previous = CURRENT;
        let next = next_ready(previous);
        QUANTUM = QUANTUM_TICKS;

        if next == previous {
            return;
        }

        if TASKS[previous].state == TaskState::Running {
            TASKS[previous].state = TaskState::Ready;
        }
        TASKS[next].state = TaskState::Running;
        CURRENT = next;
        let next_frame = TASKS[next].frame;
        irq_set_switch_frame(next_frame, TASKS[next].stack_top);
    }
}

unsafe fn next_ready(previous: usize) -> usize {
    for offset in 1..=MAX_TASKS {
        let index = (previous + offset) % MAX_TASKS;
        if TASKS[index].state == TaskState::Ready {
            return index;
        }
    }
    previous
}

pub fn yield_now() {
    unsafe {
        QUANTUM = 0;
        hlt();
    }
}

pub fn sleep(ticks: u32) {
    unsafe {
        TASKS[CURRENT].wake_at = TICKS + core::cmp::max(1, ticks) as u64;
        TASKS[CURRENT].state = TaskState::Blocked;
        QUANTUM = 0;
        while TASKS[CURRENT].state == TaskState::Blocked {
            hlt();
        }
    }
}

pub fn tick_count() -> u64 {
    unsafe { TICKS }
}

fn task_count() -> usize {
    unsafe {
        TASKS
            .iter()
            .filter(|task| task.state != TaskState::Empty)
            .count()
    }
}

pub fn current_pid() -> u32 {
    unsafe { TASKS[CURRENT].pid }
}

pub fn print_tasks() {
    println!("PID  STATE    TICKS  NAME");
    unsafe {
        for task in TASKS.iter() {
            if task.state == TaskState::Empty {
                continue;
            }
            let state = match task.state {
                TaskState::Ready => "ready",
                TaskState::Running => "running",
                TaskState::Blocked => "sleep",
                TaskState::Empty => "-",
            };
            println!(
                "{:<4} {:<8} {:<6} {}",
                task.pid, state, task.ticks, task.name
            );
        }
    }
}

#[allow(dead_code)]
extern "C" fn demo_counter() -> ! {
    unsafe {
        sti();
    }
    loop {
        sleep(10_000);
    }
}

#[allow(dead_code)]
extern "C" fn demo_worker() -> ! {
    unsafe {
        sti();
    }
    loop {
        sleep(10_000);
    }
}