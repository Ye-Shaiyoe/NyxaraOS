use core::fmt::{self, Write};
use crate::framebuffer;

extern "C" {
    fn vga_putchar(c: u8);
    fn vga_putchar_at(c: u8, color: u8, x: usize, y: usize);
    fn vga_clear();
    fn vga_set_color(fg: u8, bg: u8);
    fn vga_backspace();
    fn vga_set_cursor(x: usize, y: usize);
    fn vga_get_cursor(x: *mut usize, y: *mut usize);
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black        = 0,
    Blue         = 1,
    Green        = 2,
    Cyan         = 3,
    Red          = 4,
    Magenta      = 5,
    Brown        = 6,
    LightGray    = 7,
    DarkGray     = 8,
    LightBlue    = 9,
    LightGreen   = 10,
    LightCyan    = 11,
    LightRed     = 12,
    LightMagenta = 13,
    Yellow       = 14,
    White        = 15,
}

impl Color {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0  => Color::Black,
            1  => Color::Blue,
            2  => Color::Green,
            3  => Color::Cyan,
            4  => Color::Red,
            5  => Color::Magenta,
            6  => Color::Brown,
            7  => Color::LightGray,
            8  => Color::DarkGray,
            9  => Color::LightBlue,
            10 => Color::LightGreen,
            11 => Color::LightCyan,
            12 => Color::LightRed,
            13 => Color::LightMagenta,
            14 => Color::Yellow,
            15 => Color::White,
            _  => Color::White,
        }
    }

    /// Map a 4-bit VGA color to its Catppuccin TrueColor equivalent.
    pub fn to_rgb(self) -> u32 {
        framebuffer::COLOR_TABLE[self as usize]
    }
}

// ── Writer (dispatches to FB or VGA text mode) ───────────────────────────────

pub struct VgaWriter;

impl Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if framebuffer::is_active() {
                framebuffer::putchar(byte);
            } else {
                unsafe { vga_putchar(byte); }
            }
        }
        Ok(())
    }
}

// ── Color control ─────────────────────────────────────────────────────────────

pub fn set_color(fg: Color, bg: Color) {
    if framebuffer::is_active() {
        framebuffer::set_color(fg.to_rgb(), bg.to_rgb());
    } else {
        unsafe { vga_set_color(fg as u8, bg as u8); }
    }
}

/// Build a VGA color byte: low nibble = fg, high nibble = bg.
pub fn make_color(fg: Color, bg: Color) -> u8 {
    (fg as u8) | ((bg as u8) << 4)
}

// ── Screen operations ─────────────────────────────────────────────────────────

pub fn clear_screen() {
    if framebuffer::is_active() {
        framebuffer::clear_screen();
    } else {
        unsafe { vga_clear(); }
    }
}

pub fn backspace() {
    if framebuffer::is_active() {
        framebuffer::putchar(b'\x08');
    } else {
        unsafe { vga_backspace(); }
    }
}

pub fn get_cursor() -> (usize, usize) {
    if framebuffer::is_active() {
        framebuffer::get_cursor()
    } else {
        let mut x: usize = 0;
        let mut y: usize = 0;
        unsafe { vga_get_cursor(&mut x, &mut y); }
        (x, y)
    }
}

pub fn set_cursor(x: usize, y: usize) {
    if framebuffer::is_active() {
        framebuffer::set_cursor(x, y);
    } else {
        unsafe { vga_set_cursor(x, y); }
    }
}

pub fn putchar(c: u8) {
    if framebuffer::is_active() {
        framebuffer::putchar(c);
    } else {
        unsafe { vga_putchar(c); }
    }
}

pub fn putchar_at(c: u8, color: u8, x: usize, y: usize) {
    if framebuffer::is_active() {
        let fg = Color::from_u8(color & 0x0F).to_rgb();
        let bg = Color::from_u8((color >> 4) & 0x0F).to_rgb();
        framebuffer::putchar_at(c, fg, bg, x, y);
    } else {
        unsafe { vga_putchar_at(c, color, x, y); }
    }
}

pub fn get_dimensions() -> (usize, usize) {
    if framebuffer::is_active() {
        (framebuffer::cols(), framebuffer::rows())
    } else {
        (80, 25)
    }
}

pub fn print_fmt(args: fmt::Arguments) {
    let mut writer = VgaWriter;
    let _ = writer.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::print_fmt(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        $crate::print!($($arg)*);
        $crate::print!("\n");
    }};
}

#[macro_export]
macro_rules! print_colored {
    ($fg:expr, $bg:expr, $($arg:tt)*) => {{
        $crate::vga::set_color($fg, $bg);
        $crate::print!($($arg)*);
        $crate::vga::set_color($crate::vga::Color::White, $crate::vga::Color::Black);
    }};
}