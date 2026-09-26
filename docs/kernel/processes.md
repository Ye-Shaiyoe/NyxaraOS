# Processes, Scheduling, and Ring 3

Nyxara currently has a small round-robin scheduler and one Ring 3 proof-of-life task. This is a kernel prototype, not yet a general-purpose user process environment.

## Scheduler

The scheduler is implemented in `rust/src/process.rs`. `process::init()` creates the shell task, two Ring 0 demo tasks, and the Ring 3 demo. The task table holds up to eight tasks; each task has a 4 KB kernel stack and a saved interrupt frame.

The PIT runs at 100 Hz. On each timer interrupt, `nyxara_scheduler_tick()` updates task ticks and wakes expired sleepers. It switches to the next ready task after a five-tick quantum (about 50 ms), using the IRQ frame and `irq_set_switch_frame()` to resume the selected task. `sleep(ticks)` blocks a task until a later tick. The task table and scheduler state are global; per-process address spaces and task resource cleanup are not implemented.

## TSS and Privilege Transition

The GDT provides kernel selectors `0x08`/`0x10`, user selectors `0x1B`/`0x23` (the user descriptors at indexes 3 and 4 with RPL 3), and a 32-bit TSS at selector `0x28`. `gdt_init()` loads the TSS with `ltr`. The TSS `ss0` is `0x10` and its initial `esp0` is `0x00140000`.

When an interrupt or exception changes privilege from Ring 3 to Ring 0, the CPU takes the kernel stack pointer from the TSS and saves the user return state. The assembly stubs save the remaining registers and restore execution with `iret`. The IRQ task-switch path copies the appropriate three-word Ring 0 or five-word Ring 3 return frame to the selected task stack.

The current TSS uses a fixed initial kernel stack. Scheduler integration that updates `esp0` to a distinct kernel stack for each user task is not complete, so this must not be treated as full process isolation.

## Ring 3 Demo

The demo code is linked in `.user_text`, copied into an allocated physical frame, and mapped at `0x04000000` with `PAGE_USER`. A separate allocated frame is mapped writable at `0x04001000` for its user stack. The scheduler's initial register frame uses `CS = 0x1B`, `SS = 0x23`, and enters user mode through `iret` during an IRQ context switch.

The demo executes `int 0x80` with `SYS_GETPID`, then loops. A serial line such as `[Syscall] getpid -> 4` confirms the syscall dispatcher observed the Ring 3 task. The code and stack are mapped in the kernel's single page directory; tasks do not yet have separate page directories.

## Current Limits

- The Ring 3 task is a built-in demo, not an ELF executable.
- The scheduler shares one page directory between tasks; user memory is not isolated per process.
- The TSS kernel stack pointer is not updated on every task switch.
- `sys_exit` does not terminate a task or release its memory.
- Syscall pointers are not validated against user mappings.
- `fork`, `execve`, `waitpid`, and `brk` are not implemented.

See [System Calls](syscalls.md), [Virtual Memory](vmm.md), and [the roadmap](../development/roadmap-standards.md) for the related interfaces and remaining work.