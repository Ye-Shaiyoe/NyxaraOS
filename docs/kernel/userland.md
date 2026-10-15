# Userland: ELF Programs, `fork`, `execve` & `waitpid`

Nyxara runs real ELF32 i386 executables as Ring 3 processes, each in its own virtual address space with a dedicated kernel stack. The full Unix process life cycle — `fork`, `execve`, `exit`, `waitpid` — is implemented on top of the int `0x80` dispatcher and the round-robin scheduler.

## Address Space Layout

Every user task gets a private page directory cloned from the kernel directory (see [VMM](vmm.md)):

| Range | Purpose |
|---|---|
| `0x00000000 – 0x04000000` | Kernel identity map (shared, supervisor-only) |
| `0x04000000 – 0x08000000` | User program image (ELF `PT_LOAD` segments) |
| `0x07FFE000 – 0x08000000` | User stack, two zeroed writable pages, grows down |
| `0xC0000000 – 0xC1000000` | Kernel demand-paging zone (shared) |

Each task also owns one of the eight 4 KB kernel stacks (`STACKS` in `process.rs`). On every context switch the scheduler writes the task's stack top into `TSS.esp0` via `tss_set_stack()`, so `int 0x80` and IRQ entries from Ring 3 always land on the owning task's kernel stack. Kernel tasks (the shell) run on the boot stack at `0x00140000` and are signaled by `stack_top == 0`.

Switching to a different address space is `vmm::switch_directory(pd)`, which reloads CR3 only when the target directory differs from the active one.

## ELF32 Loader (`rust/src/elf.rs`)

`elf::load(pd_phys, image) -> Result<entry, errno>` validates the image (magic, ELFCLASS32, little-endian, `ET_EXEC`, `EM_386`) and maps every `PT_LOAD` program header into the target page directory:

- Pages are allocated, zeroed, and mapped `PAGE_USER`, plus `PAGE_WRITABLE` when the segment has `PF_W`.
- File content is copied through the identity map (frames below 64 MB are directly accessible), so the target directory never needs to be active.
- `p_memsz > p_filesz` (BSS) is covered because frames start zeroed.
- Overlapping segments and images outside the user range are rejected with `-ENOEXEC`.

Build-time: userland programs live in `userland/*.asm` (or written in NyxC like `../NyxC/hello.nyx`), linked at `0x04000000` by `user.ld`, and the resulting ELF files are embedded into the kernel with `include_bytes!`.

## Initrd: `/bin` Registration (`rust/src/initrd.rs`)

At boot, `initrd::init()` copies each embedded program image into the RamFS as `/bin/<name>`. `execve` therefore resolves binaries through the normal VFS path, and `ls /bin` lists the available programs. Current programs:

- `hello` — compiled from `NyxC/hello.nyx` via the NyxC compiler, prints `"Hello from NyxC on NyxaraOS!\n"` via `sys::write(1, ...)` and exits 0.
- `forktest` — `fork()`s; the child prints its pid and exits 42, the parent `waitpid()`s, prints the exit code, and exits 0.
- `01_hello` through `09_struct` — NyxC learning programs embedded from `NyxC/Learning-NyxC/`.

`10_globals` is kept in `NyxC/tests/` and is not auto-embedded. The kernel image
must stay below the legacy VGA aperture at `0x000A0000`; otherwise the `.bss`
 (including the heap allocator) ends up in VGA memory and is overwritten during
 early boot. Add another auto-registered example only after checking that the new
 kernel's `.bss` remains below that address.

## Process Life Cycle (`rust/src/process.rs`)

- **`spawn_elf(image, name)`** — kernel-side spawn: clone a kernel directory, load the ELF, map the stack, build a Ring 3 interrupt frame, register the task. Used by the shell.
- **`fork` (`EAX=2`, Ring 3 only)** — `vmm::clone_directory_full()` duplicates the parent's user pages eagerly (kernel tables are shared), and the child receives a copy of the caller's interrupt frame with `EAX = 0`. The parent gets the child pid.
- **`execve` (`EAX=11`, Ring 3 only)** — reads the program from the VFS, builds a fresh address space, releases the old one, reloads CR3, and rewrites the in-flight syscall frame so `iret` lands on the new entry point with a fresh stack. `argv`/`envp` are accepted but not yet delivered.
- **`exit` (`EAX=1`, Ring 3 only)** — releases the address space and marks the task `Zombie`. If the parent is blocked in `waitpid`, it is woken immediately and the exiting task is reaped at once; otherwise it stays `Zombie` until reaped. Orphaned children are re-parented to pid 0.
- **`waitpid` (`EAX=7`, Ring 3 only)** — reaps an already-exited child, or blocks (`wake_at = u64::MAX`) until one exits; the exit code is written through the parent's page tables by translating the user status pointer to a physical address.

Exiting a task from syscall context cannot return through `iret`, so `exit_current()` picks the next ready task and calls `task_resume_asm(frame, cr3)` in `boot/kernel_entry.asm`, which loads CR3, restores the general registers, and `iret`s through the saved frame — never returning.

## Shell Integration

Typing the program name directly (e.g. `hello`) or using `run <program>` looks the binary up in `/bin` (or takes an absolute VFS path), spawns it, and blocks the shell in `wait_kernel(pid)` — a polling reap loop that yields between checks. Repeated runs reuse the freed task slot and allocated frames, so the loop is leak-free.

## Verification

Headless QEMU test using the monitor (the shell reads the keyboard, not serial):

```
qemu-system-i386 -drive file=nyxara.img,format=raw -display none \
  -monitor stdio -serial file:/tmp/boot.log -netdev user,id=net0 -device rtl8139,netdev=net0
# then: sendkey r / u / n / spc / h / e / l / l / o / ret, wait, screendump
```

Expected output for `run forktest`:

```
forktest: parent forked, child pid 3
forktest: child running, my pid is 3
forktest: parent: child exited with code 42
'forktest' finished with exit code 0.
```

Serial log confirms the kernel side: `Spawned 'forktest' in ring 3 (pid=2, ...)`, `fork: task 2 cloned into pid 3`, `Task 3 exited with code 42`, `Task 2 resumed after child 3 exited`.

## Current Limits

- `fork` copies user memory eagerly; copy-on-write is not implemented.
- `execve` does not pass `argv`/`envp` and loads only from the RamFS (no block devices yet).
- User programs have no heap (`brk`/`mmap` are missing) and no stdin (`sys_read` is a stub).
- Syscall pointers for `write` are still dereferenced without mapping validation; `execve` path strings and the `waitpid` status pointer are validated.
- The identity map covers 64 MB, so frame copies rely on physical memory staying below that bound (currently 32 MB).
