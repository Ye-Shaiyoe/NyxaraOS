# Unix System Calls (`int 0x80`)

Nyxara has a small Unix-like syscall dispatcher on software interrupt vector 128 (`0x80`). The gate is callable from Ring 3, but the syscall set is incomplete and does not yet provide POSIX compatibility or safe user-memory handling.

## ABI & Register Calling Convention

Nyxara adheres to the standard Linux i386 ABI calling convention:

| Register | Purpose | Description |
|---|---|---|
| **`EAX`** | Syscall Number / Return Value | Input: System call ID. Output: Return code or negative errno. |
| **`EBX`** | Argument 1 | First parameter (e.g. file descriptor or exit code) |
| **`ECX`** | Argument 2 | Second parameter (e.g. buffer pointer) |
| **`EDX`** | Argument 3 | Third parameter (e.g. buffer length) |
| **`ESI`** | Argument 4 | Fourth parameter |
| **`EDI`** | Argument 5 | Fifth parameter |

## Declared and Implemented Calls

```rust
pub const SYS_EXIT:   u32 = 1;
pub const SYS_FORK:   u32 = 2;
pub const SYS_READ:   u32 = 3;
pub const SYS_WRITE:  u32 = 4;
pub const SYS_OPEN:   u32 = 5;
pub const SYS_CLOSE:  u32 = 6;
pub const SYS_GETPID: u32 = 20;
```

Only `SYS_EXIT`, `SYS_READ`, `SYS_WRITE`, and `SYS_GETPID` are currently dispatched. `SYS_FORK`, `SYS_OPEN`, and `SYS_CLOSE` are constants only; they fall through to `-ENOSYS` (`-38`).

### System Call Descriptions

### `sys_exit` (`EAX = 1`)
- **Arguments**: `EBX`: Exit status code (`int status`).
- **Current behavior**: Logs the requested exit code and returns. It does not terminate the task or release resources.

### `sys_read` (`EAX = 3`)
- **Arguments**: `EBX`: File descriptor (`int fd`), `ECX`: Destination buffer pointer (`void* buf`), `EDX`: Maximum bytes to read (`size_t count`).
- **Current behavior**: Stub that returns `0`; it does not read stdin or files.

### `sys_write` (`EAX = 4`)
- **Arguments**: `EBX`: File descriptor (`int fd`), `ECX`: Source buffer pointer (`const void* buf`), `EDX`: Byte count (`size_t count`).
- **Behavior**:
  - `fd == 1` (stdout) or `fd == 2` (stderr): Writes characters directly to the VGA display and serial debug log.
  - Invalid descriptors return `-EBADF` (`-9`).

`sys_write` currently dereferences the supplied buffer directly. User pointers are not checked against mapped user pages, so this interface is not safe for untrusted user programs yet.

### `sys_getpid` (`EAX = 20`)
- **Returns**: PID of the scheduler's current task.

## Trap Gate Setup (`syscall::init`)

```rust
pub fn init() {
    unsafe {
        isr_register_handler(128, syscall_dispatcher);
    }
}
```
The GDT/IDT sets vector 128 with privilege level DPL=3 (`0xEE`), allowing Ring 3 user tasks to invoke `int 0x80` without generating a General Protection Fault (`#GP`).

## Example Usage (Assembly Call)

```rust
let test_msg = "POSIX syscall test verified.\n";
unsafe {
    core::arch::asm!(
        "int 0x80",
        in("eax") 4u32,                      // SYS_WRITE
        in("ebx") 1u32,                      // stdout
        in("ecx") test_msg.as_ptr() as u32,  // Buffer pointer
        in("edx") test_msg.len() as u32,     // String length
    );
}
```

The shell command `syscall` invokes `int 0x80` from kernel context to exercise `SYS_WRITE` and `SYS_GETPID`; it does not test a Ring 3 transition. The Ring 3 demo's syscall check is described in [Processes, Scheduling, and Ring 3](processes.md).
