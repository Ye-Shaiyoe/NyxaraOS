# Shell Commands Reference

Nyxara OS includes over 30 built-in shell commands implemented in `rust/src/commands.rs`. This reference provides complete documentation for each command, including arguments, behavior, and output examples.

---

### System & Hardware Diagnostics

#### `help`
- **Syntax**: `help`
- **Description**: Displays the complete list of available built-in commands with concise usage summaries.

#### `about`
- **Syntax**: `about`
- **Description**: Displays operating system metadata, architecture specifications, language distribution (C, NASM, Rust), and licensing information.

#### `sysinfo`
- **Syntax**: `sysinfo`
- **Description**: Inspects low-level hardware state:
  - CPU Mode (32-bit Protected Mode).
  - Current Stack Pointer (`ESP`).
  - Interrupt Status (`EFLAGS.IF`).
  - Total PIT timer ticks elapsed since boot.

#### `free` / `meminfo`
- **Syntax**: `free` or `meminfo`
- **Description**: Reports memory statistics from the Physical Memory Manager (PMM) and Kernel Heap:
  - Total Physical Memory (KB / MB).
  - Used and Free Physical Frames.
  - Kernel Heap Base, Heap Size (4 MB), and Allocation State.

#### `uptime`
- **Syntax**: `uptime`
- **Description**: Prints elapsed time since system startup in days, hours, minutes, seconds, and milliseconds.

#### `date`
- **Syntax**: `date`
- **Description**: Queries the Real-Time Clock (RTC) and displays the current calendar date in `YYYY-MM-DD` format.

#### `time`
- **Syntax**: `time`
- **Description**: Queries the Real-Time Clock (RTC) and displays the current 24-hour time in `HH:MM:SS` format.

#### `lalaufetch`
- **Syntax**: `lalaufetch`
- **Description**: Neofetch-style system overview presenting a vibrant ASCII galaxy logo alongside OS name, kernel version, CPU architecture, memory utilization, display mode, and uptime.

---

### Virtual Memory & System Testing

#### `vmm [info|test]`
- **Syntax**: `vmm` or `vmm info` or `vmm test`
- **Description**:
  - `vmm info`: Displays Page Directory physical address (`CR3`), paging state, identity-mapped regions, and demand-paging counters.
  - `vmm test`: Triggers automated self-tests of demand paging by writing to unmapped virtual memory addresses `0xC0000000` and `0xC0001000`, verifying on-the-fly Page Fault allocation.

#### `syscall`
- **Syntax**: `syscall`
- **Description**: Invokes `int 0x80` from kernel context to exercise `SYS_WRITE` and `SYS_GETPID`. This command does not test a Ring 3 transition; see [Processes, Scheduling, and Ring 3](../kernel/processes.md) for the user-mode demo.

#### `panic [message]`
- **Syntax**: `panic [custom_message]`
- **Description**: Intentionally triggers a Rust kernel panic (`panic!()`). Used to test the visual crash handler, serial diagnostics output, and CPU halt mechanism.

#### `reboot`
- **Syntax**: `reboot`
- **Description**: Triggers a hardware reset by pulsing the CPU reset line via the Intel 8042 PS/2 controller (sending `0xFE` to port `0x64`).

---

### File Management (VFS)

#### `ls`
- **Syntax**: `ls`
- **Description**: Lists all files currently stored in the in-memory Virtual File System (RamFS) along with their sizes in bytes.

#### `cat <filename>`
- **Syntax**: `cat <file>`
- **Description**: Reads the specified file from VFS and prints its text content to the terminal.

#### `touch <filename>`
- **Syntax**: `touch <file>`
- **Description**: Creates a new empty file in the VFS if it does not already exist.

#### `write <filename> <text>`
- **Syntax**: `write <file> <content>`
- **Description**: Overwrites or creates `<file>` in the VFS with the provided text string.

#### `mway <filename>`
- **Syntax**: `mway <file>`
- **Description**: Opens the full-screen visual text editor for `<file>`. Supports arrow key navigation, text insertion, saving with `Ctrl+S`, and exiting with `Ctrl+Q`.

---

### Networking Commands

#### `ifconfig` / `netinfo`
- **Syntax**: `ifconfig` or `ifconfig <ip> <netmask> <gateway>`
- **Description**: Displays current network interface parameters (MAC, IP, Netmask, Gateway, DNS, Link State, RX/TX counters) or reconfigures static IP settings.

#### `dhcp`
- **Syntax**: `dhcp`
- **Description**: Initiates a DHCP DORA broadcast exchange on `eth0` to automatically obtain an IP lease, gateway, and DNS configuration from a local DHCP server.

#### `dns <hostname>` / `nslookup <hostname>`
- **Syntax**: `dns <domain>` or `nslookup <domain>`
- **Description**: Queries the configured DNS server over UDP port 53 to resolve a domain name into its corresponding IPv4 address.
- **Example**: `dns google.com`

#### `ping <host_or_ip>`
- **Syntax**: `ping <target>`
- **Description**: Transmits 4 ICMP Echo Requests (ping packets) to the destination and computes round-trip time (RTT) in milliseconds.

#### `arp [-a|-c]`
- **Syntax**: `arp` or `arp -a` or `arp -c`
- **Description**:
  - `arp -a`: Displays the current ARP cache table (IP to MAC mappings).
  - `arp -c`: Flushes all entries from the dynamic ARP cache.

#### `netstat`
- **Syntax**: `netstat`
- **Description**: Summarizes network driver statistics, interface packet counters, active socket states, and protocol layer counters.

#### `curl <url>` / `fetch <url>`
- **Syntax**: `curl <http://host[:port]/path>`
- **Description**: Connects to an HTTP server over TCP, issues a `GET` request, and displays the response status, headers, and body.
- **Example**: `curl http://10.0.2.2:8080/index.html`

#### `httpd [port]`
- **Syntax**: `httpd` or `httpd <port>`
- **Description**: Starts the embedded Nyxara HTTP web server (default port 80). Serves dynamic HTML system status dashboards to connecting browsers.

#### `nc [-u] <ip> <port>`
- **Syntax**: `nc <ip> <port>` or `nc -u <ip> <port>`
- **Description**: Netcat utility for transmitting raw text payloads over TCP or UDP (`-u`).

---

### Graphics & User Interface

#### `mouse [test]`
- **Syntax**: `mouse` or `mouse test`
- **Description**:
  - `mouse`: Displays current PS/2 mouse cursor coordinates and button state.
  - `mouse test`: Activates real-time cursor tracking display on the console (press any key to exit).

#### `paint`
- **Syntax**: `paint`
- **Description**: Launches the interactive graphical drawing canvas in the Linear Framebuffer. Allows free-hand sketching with the PS/2 mouse (press `ESC` to return to shell).

#### `color <fg> <bg>`
- **Syntax**: `color <foreground_0..15> <background_0..15>`
- **Description**: Dynamically changes the active console text color palette.

#### `clear`
- **Syntax**: `clear`
- **Description**: Clears the console display buffer, resets cursor position to `(0, 0)`, and reprints the Nyxara banner.

#### `echo <text>`
- **Syntax**: `echo <text>`
- **Description**: Echoes the input string back to standard output.

#### `calc <a op b>`
- **Syntax**: `calc <number> <operator> <number>`
- **Description**: Evaluates basic integer arithmetic operations (`+`, `-`, `*`, `/`, `%`).
- **Example**: `calc 42 * 10`
