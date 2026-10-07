use crate::vga::Color;
use crate::{print, println, print_colored};
use super::utils::print_pad_right;

extern "C" {
    fn keyboard_has_char() -> bool;
    fn keyboard_getchar() -> u16;
}

pub fn cmd_ifconfig(args: &str) {
    let trimmed = args.trim();
    if trimmed.starts_with("set ") {
        let parts: alloc::vec::Vec<&str> = trimmed[4..].split_whitespace().collect();
        if parts.len() == 3 {
            let ip_opt = crate::net::parse_ip(parts[0]);
            let mask_opt = crate::net::parse_ip(parts[1]);
            let gw_opt = crate::net::parse_ip(parts[2]);

            if let (Some(ip), Some(mask), Some(gw)) = (ip_opt, mask_opt, gw_opt) {
                crate::net::update_config(ip, mask, gw);
                print_colored!(Color::LightGreen, Color::Black, "[OK] ");
                println!("Network eth0 updated: IP={}, Mask={}, Gateway={}",
                    parts[0], parts[1], parts[2]);
                println!("Configuration saved to /etc/network.conf in VFS.");
                return;
            }
        }
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("ifconfig set <ip> <netmask> <gateway>");
        println!("Example: ifconfig set 10.0.2.15 255.255.255.0 10.0.2.2");
        return;
    }

    if !trimmed.is_empty() && trimmed != "eth0" {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("ifconfig [eth0]");
        println!("       ifconfig set <ip> <netmask> <gateway>");
        return;
    }

    match crate::net::get_config() {
        Some(cfg) => {
            let (rx_pkts, tx_pkts, rx_bytes, tx_bytes) = crate::net::get_stats();
            let status = if cfg.is_up { "UP, BROADCAST, RUNNING" } else { "DOWN" };

            print_colored!(Color::LightCyan, Color::Black, "eth0: ");
            println!("flags=<{}> mtu 1500", status);
            println!("      inet {}  netmask {}  gateway {}",
                crate::net::format_ip(&cfg.ip),
                crate::net::format_ip(&cfg.netmask),
                crate::net::format_ip(&cfg.gateway));
            println!("      nameserver {}", crate::net::format_ip(&cfg.dns));
            println!("      ether {} (Realtek RTL8139)", crate::net::format_mac(&cfg.mac));
            println!("      RX packets {}  bytes {} ({})",
                rx_pkts, rx_bytes, if rx_bytes < 1024 { "B" } else { "KB" });
            println!("      TX packets {}  bytes {} ({})",
                tx_pkts, tx_bytes, if tx_bytes < 1024 { "B" } else { "KB" });
        }
        None => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("Network interface eth0 unavailable.");
        }
    }
}

pub fn cmd_ping(args: &str) {
    let host = args.trim();
    if host.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("ping <host_or_ip> (e.g. ping 10.0.2.2 or ping google.com)");
        return;
    }

    let target_ip = match crate::net::parse_ip(host) {
        Some(ip) => ip,
        None => {
            print_colored!(Color::LightCyan, Color::Black, "[DNS] ");
            println!("Resolving host '{}'...", host);
            match crate::net::dns_resolve(host, 3000) {
                Ok(ip) => ip,
                Err(e) => {
                    print_colored!(Color::LightRed, Color::Black, "Error: ");
                    println!("Failed to resolve '{}': {}", host, e);
                    return;
                }
            }
        }
    };

    println!("PING {} ({}) 32 bytes of ICMP data:", host, crate::net::format_ip(&target_ip));

    let mut transmitted = 0;
    let mut received = 0;

    for seq in 1..=4 {
        transmitted += 1;
        match crate::net::send_ping(target_ip, seq, 1500) {
            Ok(rtt) => {
                received += 1;
                println!("32 bytes from {}: icmp_seq={} ttl=64 time={} ms",
                    crate::net::format_ip(&target_ip), seq, rtt);
            }
            Err(e) => {
                println!("From {}: icmp_seq={} {}", host, seq, e);
            }
        }
    }

    println!("\n--- {} ping statistics ---", host);
    let loss = if transmitted > 0 { ((transmitted - received) * 100) / transmitted } else { 0 };
    println!("{} packets transmitted, {} received, {}% packet loss", transmitted, received, loss);
}

pub fn cmd_dhcp() {
    print_colored!(Color::LightCyan, Color::Black, "[DHCP] ");
    println!("Requesting dynamic IP configuration via DHCP (DORA)...");

    match crate::net::request_lease() {
        Ok(lease) => {
            print_colored!(Color::LightGreen, Color::Black, "[OK] ");
            println!("DHCP lease acquired successfully:");
            println!("      Assigned IP : {}", crate::net::format_ip(&lease.ip));
            println!("      Subnet Mask : {}", crate::net::format_ip(&lease.netmask));
            println!("      Gateway     : {}", crate::net::format_ip(&lease.gateway));
            println!("      DNS Server  : {}", crate::net::format_ip(&lease.dns));
            println!("      Lease Time  : {} seconds", lease.lease_sec);
            println!("Configuration applied and saved to /etc/network.conf.");
        }
        Err(e) => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("DHCP transaction failed: {}", e);
        }
    }
}

pub fn cmd_dns(args: &str) {
    let domain = args.trim();
    if domain.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("dns <hostname> (e.g. dns google.com)");
        return;
    }

    print_colored!(Color::LightCyan, Color::Black, "[DNS] ");
    println!("Querying DNS for '{}'...", domain);

    match crate::net::dns_resolve(domain, 3000) {
        Ok(ip) => {
            print_colored!(Color::LightGreen, Color::Black, "[OK] ");
            println!("{} has IPv4 address {}", domain, crate::net::format_ip(&ip));
        }
        Err(e) => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("DNS lookup failed for '{}': {}", domain, e);
        }
    }
}

pub fn cmd_arp(args: &str) {
    let trimmed = args.trim();
    if trimmed == "-c" || trimmed == "clear" {
        crate::net::arp::flush_cache();
        print_colored!(Color::LightGreen, Color::Black, "[OK] ");
        println!("ARP cache table cleared.");
        return;
    }

    let table = crate::net::arp::get_table();
    print_colored!(Color::LightCyan, Color::Black, "Dynamic ARP Table Entries:\n");
    if table.is_empty() {
        println!("  (ARP cache is empty)");
        return;
    }

    print!("  ");
    print_pad_right("IP Address", 16);
    print!(" ");
    print_pad_right("HW Address", 18);
    println!(" Age");
    println!("  --------------------------------------------------");
    let now = unsafe { crate::net::timer_get_uptime_ms() };
    for entry in table {
        let age_sec = (now.saturating_sub(entry.updated_ms)) / 1000;
        let ip_str = crate::net::format_ip(&entry.ip);
        let mac_str = crate::net::format_mac(&entry.mac);
        print!("  ");
        print_pad_right(&ip_str, 16);
        print!(" ");
        print_pad_right(&mac_str, 18);
        println!(" {}s", age_sec);
    }
}

pub fn cmd_netstat() {
    print_colored!(Color::LightCyan, Color::Black, "Network Subsystem Status:\n");
    if let Some(cfg) = crate::net::get_config() {
        let status = if cfg.is_up { "UP, RUNNING" } else { "DOWN" };
        let (rx_pkts, tx_pkts, rx_bytes, tx_bytes) = crate::net::get_stats();
        let arp_count = crate::net::arp::get_table().len();
        let tcp_sockets = crate::net::tcp::get_socket_count();

        println!("  Interface       : eth0 (Realtek RTL8139 PCI)");
        println!("  Link Status     : {}", status);
        println!("  MAC Address     : {}", crate::net::format_mac(&cfg.mac));
        println!("  IPv4 Address    : {}", crate::net::format_ip(&cfg.ip));
        println!("  Subnet Mask     : {}", crate::net::format_ip(&cfg.netmask));
        println!("  Default Gateway : {}", crate::net::format_ip(&cfg.gateway));
        println!("  DNS Server      : {}", crate::net::format_ip(&cfg.dns));
        println!("  Traffic Stats   : RX {} pkts ({} B), TX {} pkts ({} B)",
            rx_pkts, rx_bytes, tx_pkts, tx_bytes);
        println!("  Active Sockets  : {} TCP sockets", tcp_sockets);
        println!("  ARP Entries     : {} cached", arp_count);
    } else {
        println!("  No active network interfaces detected.");
    }
}

pub fn cmd_curl(args: &str) {
    let url = args.trim();
    if url.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("curl <url> (e.g. curl http://10.0.2.2:8000/ or curl 10.0.2.2)");
        return;
    }

    print_colored!(Color::LightCyan, Color::Black, "[HTTP] ");
    println!("Fetching '{}'...", url);

    match crate::net::fetch(url, 5000) {
        Ok(resp) => {
            println!("{}", resp);
        }
        Err(e) => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("HTTP request failed: {}", e);
        }
    }
}

pub fn cmd_httpd(args: &str) {
    let port: u16 = args.trim().parse().unwrap_or(8080);
    if crate::net::tcp::listen(port).is_err() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Failed to bind HTTP server to port {}.", port);
        return;
    }

    let cfg = crate::net::get_config();
    let ip_str = cfg.map(|c| crate::net::format_ip(&c.ip)).unwrap_or_else(|| alloc::string::String::from("0.0.0.0"));

    print_colored!(Color::LightGreen, Color::Black, "[HTTPD] ");
    println!("Nyxara Micro HTTP Server listening on {}:{}", ip_str, port);
    println!("Routes available: / (Dashboard), /api/status, or VFS files.");
    println!("Press Ctrl+C or 'q' to stop server.\n");

    loop {
        crate::net::poll();
        if crate::net::tcp::service_http_server(port) {
            println!("[HTTPD] Request serviced.");
        }

        if unsafe { keyboard_has_char() } {
            let key = unsafe { keyboard_getchar() };
            if key == b'q' as u16 || key == b'Q' as u16 || key == crate::shell::KEY_CTRL_C {
                println!("[HTTPD] Server stopped by user.");
                crate::net::tcp::close_listener(port);
                break;
            }
        }

        unsafe { core::arch::asm!("hlt"); }
    }
}

pub fn cmd_nc(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.trim().split_whitespace().collect();
    if parts.len() < 2 {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("nc [-u] <ip> <port> [message]");
        println!("Example: nc -u 10.0.2.2 12345 Hello");
        return;
    }

    let (is_udp, target_ip_str, port_str, msg_start_idx) = if parts[0] == "-u" {
        if parts.len() < 3 {
            println!("Usage: nc -u <ip> <port> [message]");
            return;
        }
        (true, parts[1], parts[2], 3)
    } else {
        (false, parts[0], parts[1], 2)
    };

    let target_ip = match crate::net::parse_ip(target_ip_str) {
        Some(ip) => ip,
        None => {
            println!("Invalid target IP '{}'.", target_ip_str);
            return;
        }
    };

    let port = match port_str.parse::<u16>() {
        Ok(p) => p,
        Err(_) => {
            println!("Invalid port number '{}'.", port_str);
            return;
        }
    };

    let message = if parts.len() > msg_start_idx {
        parts[msg_start_idx..].join(" ")
    } else {
        alloc::string::String::from("Hello from Nyxara Netcat!")
    };

    if is_udp {
        match crate::net::udp::send_to(target_ip, 49152, port, message.as_bytes()) {
            Ok(_) => println!("Sent {} UDP bytes to {}:{}.", message.len(), target_ip_str, port),
            Err(e) => println!("Error sending UDP: {}", e),
        }
    } else {
        match crate::net::tcp::connect(target_ip, port, 3000) {
            Ok(mut stream) => {
                let _ = stream.write(message.as_bytes());
                println!("Connected and sent {} TCP bytes. Awaiting response...", message.len());
                let resp = stream.read(2000).unwrap_or_default();
                if !resp.is_empty() {
                    println!("Received: {}", alloc::string::String::from_utf8_lossy(&resp));
                }
                stream.close();
            }
            Err(e) => println!("TCP connection failed: {}", e),
        }
    }
}
