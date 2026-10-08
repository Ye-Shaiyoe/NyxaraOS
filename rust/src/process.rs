use crate::println;
use crate::syscall::Registers;
use crate::vmm::PAGE_SIZE;

const MAX_TASKS: usize = 8;
const STACK_SIZE: usize = 4096;
const QUANTUM_TICKS: u32 = 5;
const USER_CODE_ADDR: usize = 0x04000000;
const USER_STACK_ADDR: usize = 0x04001000;
const USER_STACK_TOP: usize = 0x08000000;
const USER_STACK_PAGES: usize = 2;
const KERNEL_STACK_TOP: u32 = 0x00140000;
const USER_CODE_SELECTOR: u32 = 0x1B;
const USER_DATA_SELECTOR: u32 = 0x23;
const EPERM: i32 = -1;
const ENOMEM: i32 = -12;
const EAGAIN: i32 = -11;
const ECHILD: i32 = -10;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
    Zombie,
}

#[derive(Copy, Clone)]
struct Task {
    pid: u32,
    parent_pid: u32,
    state: TaskState,
    frame: *mut Registers,
    stack_top: u32,
    page_dir: u32,
    wake_at: u64,
    ticks: u64,
    name: [u8; 16],
    wait_target: i32,
    wait_status: u32,
    woken: i32,
    exit_code: i32,
    fds: crate::fd::FdTable,
}

impl Task {
    const fn empty() -> Self {
        Self {
            pid: 0,
            parent_pid: 0,
            state: TaskState::Empty,
            frame: core::ptr::null_mut(),
            stack_top: 0,
            page_dir: 0,
            wake_at: 0,
            ticks: 0,
            name: [0; 16],
            wait_target: 0,
            wait_status: 0,
            woken: 0,
            exit_code: 0,
            fds: crate::fd::FdTable::new(),
        }
    }
}

static mut TASKS: [Task; MAX_TASKS] = [Task::empty(); MAX_TASKS];
static mut TASK_STACKS: [usize; MAX_TASKS] = [0; MAX_TASKS];

fn get_stack_top(slot: usize) -> usize {
    unsafe {
        if TASK_STACKS[slot] == 0 {
            let frame = crate::pmm::alloc_frame().expect("No physical frame for task stack");
            TASK_STACKS[slot] = frame;
        }
        TASK_STACKS[slot] + STACK_SIZE
    }
}
static mut CURRENT: usize = 0;
static mut NEXT_PID: u32 = 1;
static mut QUANTUM: u32 = QUANTUM_TICKS;
static mut TICKS: u64 = 0;
static mut INITIALIZED: bool = false;

extern "C" {
    fn irq_set_switch_frame(regs: *mut Registers, stack_top: u32);
    fn tss_set_stack(ss0: u32, esp0: u32);
    fn task_resume_asm(frame: *const Registers, cr3: u32);
    fn hlt();
    fn sti();
    static user_demo_start: u8;
    static user_demo_end: u8;
}

fn name_bytes(name: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, &b) in name.as_bytes().iter().take(16).enumerate() {
        out[i] = b;
    }
    out
}

fn task_name(name: &[u8; 16]) -> &str {
    let len = name.iter().position(|&b| b == 0).unwrap_or(16);
    core::str::from_utf8(&name[..len]).unwrap_or("-")
}

fn pd_of(page_dir: u32) -> usize {
    if page_dir == 0 {
        crate::vmm::PAGE_DIR_PHYS
    } else {
        page_dir as usize
    }
}

fn free_slot() -> Option<usize> {
    unsafe {
        for i in 0..MAX_TASKS {
            if TASKS[i].state == TaskState::Empty {
                return Some(i);
            }
        }
    }
    None
}

pub fn init() {
    unsafe {
        TASKS[0] = Task {
            pid: NEXT_PID,
            state: TaskState::Running,
            name: name_bytes("shell"),
            ..Task::empty()
        };
        NEXT_PID += 1;
        INITIALIZED = true;
    }

    crate::logln!(
        "[Scheduler] Round-robin scheduler initialized ({} tasks).",
        task_count()
    );
}

fn build_user_frame(slot: usize, eip: usize) -> *mut Registers {
    let stack_top = get_stack_top(slot);
    let frame =
        (stack_top & !0xF).wrapping_sub(core::mem::size_of::<Registers>()) as *mut Registers;
    unsafe {
        core::ptr::write_bytes(frame as *mut u8, 0, core::mem::size_of::<Registers>());
        (*frame).ds = USER_DATA_SELECTOR;
        (*frame).eip = eip as u32;
        (*frame).cs = USER_CODE_SELECTOR;
        (*frame).eflags = 0x202;
        (*frame).useresp = (USER_STACK_TOP - 64) as u32;
        (*frame).ss = USER_DATA_SELECTOR;
    }
    frame
}

fn register_task(slot: usize, parent_pid: u32, page_dir: usize, frame: *mut Registers, name: &[u8; 16]) -> u32 {
    let pid = unsafe { NEXT_PID };
    unsafe {
        NEXT_PID += 1;
        TASKS[slot] = Task {
            pid,
            parent_pid,
            state: TaskState::Ready,
            frame,
            stack_top: get_stack_top(slot) as u32,
            page_dir: page_dir as u32,
            name: *name,
            ..Task::empty()
        };
    }
    pid
}

/// Start a new Ring 3 task from an ELF32 image in its own address space.
pub fn spawn_elf(image: &[u8], name: &str) -> Result<u32, i32> {
    let slot = match free_slot() {
        Some(slot) => slot,
        None => return Err(EAGAIN),
    };
    let pd = match crate::vmm::clone_kernel_directory() {
        Some(pd) => pd,
        None => return Err(ENOMEM),
    };
    let entry = match prepare_address_space(pd, image) {
        Ok(entry) => entry,
        Err(err) => {
            destroy_address_space(pd);
            return Err(err);
        }
    };

    let frame = build_user_frame(slot, entry);
    let pid = register_task(slot, 0, pd, frame, &name_bytes(name));
    crate::logln!(
        "[Process] Spawned '{}' in ring 3 (pid={}, pd=0x{:08X}).",
        name,
        pid,
        pd
    );
    Ok(pid)
}

#[allow(dead_code)]
unsafe fn spawn(entry: extern "C" fn() -> !, name: &'static str) -> u32 {
    let slot = match free_slot() {
        Some(slot) => slot,
        None => return 0,
    };

    let stack_top = get_stack_top(slot);
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
        name: name_bytes(name),
        ..Task::empty()
    };
    pid
}

#[allow(dead_code)]
unsafe fn spawn_user_demo() -> u32 {
    let code_size = (&user_demo_end as *const u8 as usize)
        .saturating_sub(&user_demo_start as *const u8 as usize);
    if code_size == 0 || code_size > PAGE_SIZE {
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
    core::ptr::write_bytes(stack_frame as *mut u8, 0, PAGE_SIZE);

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

    let slot = match free_slot() {
        Some(slot) => slot,
        None => return 0,
    };
    let stack_top = get_stack_top(slot);
    let frame =
        (stack_top & !0xF).wrapping_sub(core::mem::size_of::<Registers>()) as *mut Registers;
    core::ptr::write_bytes(frame as *mut u8, 0, core::mem::size_of::<Registers>());

    (*frame).ds = USER_DATA_SELECTOR;
    (*frame).eip = USER_CODE_ADDR as u32;
    (*frame).cs = USER_CODE_SELECTOR;
    (*frame).eflags = 0x202;
    (*frame).useresp = (USER_STACK_ADDR + PAGE_SIZE - 16) as u32;
    (*frame).ss = USER_DATA_SELECTOR;

    let pid = NEXT_PID;
    NEXT_PID += 1;
    TASKS[slot] = Task {
        pid,
        state: TaskState::Ready,
        frame,
        stack_top: stack_top as u32,
        name: name_bytes("ring3-demo"),
        ..Task::empty()
    };
    crate::logln!("[Process] Ring 3 demo task created (pid={}).", pid);
    pid
}

fn prepare_address_space(pd: usize, image: &[u8]) -> Result<usize, i32> {
    let entry = crate::elf::load(pd, image)?;
    for i in 0..USER_STACK_PAGES {
        let page = USER_STACK_TOP - (i + 1) * PAGE_SIZE;
        let frame = match crate::pmm::alloc_frame() {
            Some(frame) => frame,
            None => return Err(ENOMEM),
        };
        unsafe {
            core::ptr::write_bytes(frame as *mut u8, 0, PAGE_SIZE);
        }
        if crate::vmm::map_page_in(
            pd,
            page,
            frame,
            crate::vmm::PAGE_USER | crate::vmm::PAGE_WRITABLE,
        )
        .is_err()
        {
            crate::pmm::free_frame(frame);
            return Err(ENOMEM);
        }
    }
    Ok(entry)
}

fn destroy_address_space(pd: usize) {
    crate::vmm::free_user_space(pd);
    crate::pmm::free_frame(pd);
}

/// fork(2): duplicate the caller. Returns the child pid, 0 in the child.
pub fn fork_current(regs: &Registers) -> i32 {
    if regs.cs & 3 != 3 {
        return EPERM;
    }
    let cur = unsafe { CURRENT };
    let slot = match free_slot() {
        Some(slot) => slot,
        None => return EAGAIN,
    };
    let (parent_pid, parent_pd) = unsafe { (TASKS[cur].pid, TASKS[cur].page_dir) };
    let child_pd = match crate::vmm::clone_directory_full(pd_of(parent_pd)) {
        Some(pd) => pd,
        None => return ENOMEM,
    };

    let stack_top = get_stack_top(slot);
    let frame =
        (stack_top & !0xF).wrapping_sub(core::mem::size_of::<Registers>()) as *mut Registers;
    unsafe {
        *frame = *regs;
        (*frame).eax = 0;
    }

    let pid = unsafe { NEXT_PID };
    unsafe {
        NEXT_PID += 1;
        let name = TASKS[cur].name;
        TASKS[slot] = Task {
            pid,
            parent_pid,
            state: TaskState::Ready,
            frame,
            stack_top: stack_top as u32,
            page_dir: child_pd as u32,
            name,
            ..Task::empty()
        };
    }
    crate::logln!(
        "[Process] fork: task {} cloned into pid {} (pd=0x{:08X}).",
        parent_pid,
        pid,
        child_pd
    );
    pid as i32
}

/// execve(2): replace the caller's address space with an ELF32 image.
pub fn exec_current(regs: &mut Registers, image: &[u8], name: &str) -> i32 {
    if regs.cs & 3 != 3 {
        return EPERM;
    }
    let cur = unsafe { CURRENT };
    let old_pid = unsafe { TASKS[cur].pid };
    let new_pd = match crate::vmm::clone_kernel_directory() {
        Some(pd) => pd,
        None => return ENOMEM,
    };
    let entry = match prepare_address_space(new_pd, image) {
        Ok(entry) => entry,
        Err(err) => {
            destroy_address_space(new_pd);
            return err;
        }
    };

    let old_pd = unsafe { TASKS[cur].page_dir };
    if old_pd != 0 {
        destroy_address_space(old_pd as usize);
    }
    crate::vmm::switch_directory(new_pd);
    unsafe {
        TASKS[cur].page_dir = new_pd as u32;
        TASKS[cur].name = name_bytes(name);
    }

    regs.ds = USER_DATA_SELECTOR;
    regs.eax = 0;
    regs.eip = entry as u32;
    regs.cs = USER_CODE_SELECTOR;
    regs.eflags = 0x202;
    regs.useresp = (USER_STACK_TOP - 64) as u32;
    regs.ss = USER_DATA_SELECTOR;
    crate::logln!(
        "[Process] exec: task {} now runs '{}' (entry 0x{:08X}, pd=0x{:08X}).",
        old_pid,
        name,
        entry,
        new_pd
    );
    0
}

/// exit(2): never returns; either resumes the next task or idles forever.
pub fn exit_current(code: i32) -> ! {
    let cur = unsafe { CURRENT };
    let (pid, parent_pid, page_dir) =
        unsafe { (TASKS[cur].pid, TASKS[cur].parent_pid, TASKS[cur].page_dir) };
    crate::logln!("[Process] Task {} exited with code {}.", pid, code);

    let mut reaped_by_parent = false;
    unsafe {
        if parent_pid != 0 {
            for i in 0..MAX_TASKS {
                if TASKS[i].state != TaskState::Blocked || TASKS[i].pid != parent_pid {
                    continue;
                }
                if TASKS[i].wait_target == pid as i32 || TASKS[i].wait_target == -1 {
                    write_user_i32(TASKS[i].page_dir, TASKS[i].wait_status as *mut i32, code);
                    TASKS[i].woken = pid as i32;
                    TASKS[i].state = TaskState::Ready;
                    reaped_by_parent = true;
                    break;
                }
            }
        }

        for i in 0..MAX_TASKS {
            if TASKS[i].state == TaskState::Empty || TASKS[i].parent_pid != pid {
                continue;
            }
            if TASKS[i].state == TaskState::Zombie {
                TASKS[i].state = TaskState::Empty;
            } else {
                TASKS[i].parent_pid = 0;
            }
        }

        if page_dir != 0 {
            destroy_address_space(page_dir as usize);
        }
        TASKS[cur].fds.close_all();
        TASKS[cur].exit_code = code;
        TASKS[cur].state = if reaped_by_parent {
            TaskState::Empty
        } else {
            TaskState::Zombie
        };
    }

    let next = unsafe { next_ready(cur) };
    if next == cur {
        unsafe {
            loop {
                sti();
                hlt();
            }
        }
    }

    unsafe {
        TASKS[next].state = TaskState::Running;
        CURRENT = next;
        let next_pd = pd_of(TASKS[next].page_dir);
        crate::vmm::switch_directory(next_pd);
        tss_set_stack(
            0x10,
            if TASKS[next].stack_top == 0 {
                KERNEL_STACK_TOP
            } else {
                TASKS[next].stack_top
            },
        );
        let frame = TASKS[next].frame;
        task_resume_asm(frame, next_pd as u32);
    }
    unreachable!("task_resume_asm must not return")
}

/// waitpid(2): reap an exited child or block until one exits.
pub fn wait_current(regs: &mut Registers, pid: i32, status: *mut i32) -> i32 {
    if regs.cs & 3 != 3 {
        return EPERM;
    }
    let cur = unsafe { CURRENT };
    let my_pid = unsafe { TASKS[cur].pid };

    unsafe {
        for i in 0..MAX_TASKS {
            let task = TASKS[i];
            if task.state != TaskState::Zombie || task.parent_pid != my_pid {
                continue;
            }
            if pid != -1 && pid != task.pid as i32 {
                continue;
            }
            let (child, code) = (task.pid, task.exit_code);
            write_user_i32(TASKS[cur].page_dir, status, code);
            TASKS[i].state = TaskState::Empty;
            crate::logln!(
                "[Process] Task {} reaped child {} (exit code {}).",
                my_pid,
                child,
                code
            );
            return child as i32;
        }

        let mut has_living = false;
        for i in 0..MAX_TASKS {
            let task = TASKS[i];
            if task.state == TaskState::Empty
                || task.state == TaskState::Zombie
                || task.parent_pid != my_pid
            {
                continue;
            }
            if pid != -1 && pid != task.pid as i32 {
                continue;
            }
            has_living = true;
            break;
        }
        if !has_living {
            return ECHILD;
        }

        TASKS[cur].wait_target = pid;
        TASKS[cur].wait_status = status as u32;
        TASKS[cur].wake_at = u64::MAX;
        TASKS[cur].frame = regs;
        TASKS[cur].state = TaskState::Blocked;
        loop {
            sti();
            hlt();
            if TASKS[cur].state != TaskState::Blocked {
                break;
            }
        }
        let woken = TASKS[cur].woken;
        crate::logln!("[Process] Task {} resumed after child {} exited.", my_pid, woken);
        woken
    }
}

fn write_user_i32(page_dir: u32, ptr: *mut i32, value: i32) {
    if ptr.is_null() {
        return;
    }
    let pd = pd_of(page_dir);
    let bytes = value.to_le_bytes();
    for (k, &byte) in bytes.iter().enumerate() {
        let Some(phys) = crate::vmm::get_phys_addr_in(pd, ptr as usize + k) else {
            return;
        };
        unsafe {
            (phys as *mut u8).write_volatile(byte);
        }
    }
}

/// Kernel-side blocking wait used by the shell; reaps the zombie when done.
pub fn wait_kernel(pid: u32) -> i32 {
    loop {
        let mut found = false;
        unsafe {
            for i in 0..MAX_TASKS {
                if TASKS[i].pid != pid || TASKS[i].state == TaskState::Empty {
                    continue;
                }
                found = true;
                if TASKS[i].state == TaskState::Zombie {
                    let code = TASKS[i].exit_code;
                    TASKS[i].state = TaskState::Empty;
                    crate::logln!(
                        "[Process] Shell reaped child {} (exit code {}).",
                        pid,
                        code
                    );
                    return code;
                }
                break;
            }
        }
        if !found {
            return -1;
        }
        yield_now();
    }
}

#[no_mangle]
pub extern "C" fn nyxara_scheduler_tick(regs: *mut Registers) {
    unsafe {
        if !INITIALIZED {
            return;
        }

        TICKS += 1;
        TASKS[CURRENT].frame = regs;
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
        switch_to(next);
        let next_frame = TASKS[next].frame;
        irq_set_switch_frame(next_frame, TASKS[next].stack_top);
    }
}

unsafe fn switch_to(slot: usize) {
    let pd = pd_of(TASKS[slot].page_dir);
    crate::vmm::switch_directory(pd);
    tss_set_stack(
        0x10,
        if TASKS[slot].stack_top == 0 {
            KERNEL_STACK_TOP
        } else {
            TASKS[slot].stack_top
        },
    );
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
        (0..MAX_TASKS)
            .filter(|i| TASKS[*i].state != TaskState::Empty)
            .count()
    }
}

pub fn current_pid() -> u32 {
    unsafe { TASKS[CURRENT].pid }
}

pub fn print_tasks() {
    println!("PID  PPID  STATE    TICKS  NAME");
    unsafe {
        for task in TASKS.iter() {
            if task.state == TaskState::Empty {
                continue;
            }
            let state = match task.state {
                TaskState::Ready => "ready",
                TaskState::Running => "running",
                TaskState::Blocked => "sleep",
                TaskState::Zombie => "zombie",
                TaskState::Empty => "-",
            };
            println!(
                "{:<4} {:<5} {:<8} {:<6} {}",
                task.pid,
                task.parent_pid,
                state,
                task.ticks,
                task_name(&task.name)
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

pub fn current_open(path: &str, flags: u32) -> Result<i32, i32> {
    unsafe { TASKS[CURRENT].fds.open(path, flags) }
}

pub fn current_close(fd: i32) -> Result<(), i32> {
    unsafe { TASKS[CURRENT].fds.close(fd) }
}

pub fn current_fd_get(fd: i32) -> Option<crate::fd::FileDesc> {
    unsafe { TASKS[CURRENT].fds.get(fd).copied() }
}

pub fn current_fd_advance(fd: i32, count: usize) {
    unsafe {
        if let Some(desc) = TASKS[CURRENT].fds.get_mut(fd) {
            desc.offset += count;
        }
    }
}
