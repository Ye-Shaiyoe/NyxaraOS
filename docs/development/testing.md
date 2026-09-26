# Testing Nyxara

Nyxara has host-run Rust tests for selected modules, kernel self-tests for virtual memory, and QEMU smoke tests. These checks cover different layers and are not a substitute for one another.

## Host Tests

Run the available test targets from the repository root:

```bash
make test-line-editor
make test-vfs
```

Each target compiles its test file with `rustc --test` and runs the resulting executable. There is currently no aggregate `make test` target.

## Kernel Build and Boot

Build the boot image with:

```bash
make
```

For a serial-visible QEMU boot, run:

```bash
make run-serial
```

The kernel runs `vmm::test_vmm()` during startup. A successful run logs `[VMM Test] All VMM tests PASSED!`. The Ring 3 proof-of-life task should also log its creation and a `getpid` result, for example `[Syscall] getpid -> 4`. A page fault or general protection fault indicates the privilege-transition smoke test failed.

`make run-serial` is interactive and continues until QEMU is closed. For repeatable automated checks, a future test harness should capture serial output and assert expected markers and failure strings.

## Current Coverage Limits

- Host tests cover the line editor and VFS only.
- The VMM test is an in-kernel boot self-test, not a host unit test.
- Ring 3 is checked by observing serial output in QEMU; there is no automated QEMU test target yet.
- There are no automated tests for scheduler fairness, invalid user pointers, separate address spaces, or ELF loading.