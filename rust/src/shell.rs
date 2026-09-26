use crate::commands;
use crate::line_editor::{LineEditor, MAX_LINE_LEN};
use crate::vga::{self, Color};
use crate::{print_colored, println};
use alloc::string::String;
use alloc::vec::Vec;

extern "C" {
    fn keyboard_getchar() -> u16;
    fn keyboard_has_char() -> bool;
}

// Special extended keycodes matching hal/keyboard.h
pub const KEY_UP: u16 = 0x0100;
pub const KEY_DOWN: u16 = 0x0101;
pub const KEY_LEFT: u16 = 0x0102;
pub const KEY_RIGHT: u16 = 0x0103;
pub const KEY_HOME: u16 = 0x0104;
pub const KEY_END: u16 = 0x0105;
pub const KEY_PAGE_UP: u16 = 0x0106;
pub const KEY_PAGE_DOWN: u16 = 0x0107;
pub const KEY_INSERT: u16 = 0x0108;
pub const KEY_DELETE: u16 = 0x0109;
pub const KEY_CTRL_LEFT: u16 = 0x010A;
pub const KEY_CTRL_RIGHT: u16 = 0x010B;
pub const KEY_ALT_DELETE: u16 = 0x010C;
pub const KEY_ALT_BACKSPACE: u16 = 0x010D;
pub const KEY_SHIFT_LEFT: u16 = 0x010E;
pub const KEY_SHIFT_RIGHT: u16 = 0x010F;
pub const KEY_SHIFT_HOME: u16 = 0x0110;
pub const KEY_SHIFT_END: u16 = 0x0111;
pub const KEY_SHIFT_TAB: u16 = 0x0112;

// Control character constants
pub const KEY_CTRL_A: u16 = 0x0001;
pub const KEY_CTRL_C: u16 = 0x0003;
pub const KEY_CTRL_D: u16 = 0x0004;
pub const KEY_CTRL_E: u16 = 0x0005;
pub const KEY_BACKSPACE: u16 = 0x0008;
pub const KEY_TAB: u16 = 0x0009;
pub const KEY_ENTER: u16 = 0x000A;
pub const KEY_CTRL_K: u16 = 0x000B;
pub const KEY_CTRL_L: u16 = 0x000C;
pub const KEY_RETURN: u16 = 0x000D;
pub const KEY_CTRL_U: u16 = 0x0015;
pub const KEY_CTRL_W: u16 = 0x0017;
pub const KEY_CTRL_S: u16 = 0x0013;
pub const KEY_CTRL_Q: u16 = 0x0011;
pub const KEY_CTRL_R: u16 = 0x0012;
pub const KEY_DEL_CHAR: u16 = 0x007F;

const HISTORY_CAPACITY: usize = 16;
const HISTORY_FILE: &str = ".nyxara_history";

struct CommandHistory {
    entries: [[u8; MAX_LINE_LEN]; HISTORY_CAPACITY],
    lens: [usize; HISTORY_CAPACITY],
    count: usize,
}

impl CommandHistory {
    const fn new() -> Self {
        Self {
            entries: [[0; MAX_LINE_LEN]; HISTORY_CAPACITY],
            lens: [0; HISTORY_CAPACITY],
            count: 0,
        }
    }

    fn load() -> Self {
        let mut history = Self::new();
        if let Some(data) = crate::vfs::read_file(HISTORY_FILE) {
            let mut start = 0;
            for end in 0..=data.len() {
                if end == data.len() || data[end] == b'\n' {
                    history.push(&data[start..end]);
                    start = end + 1;
                }
            }
        }
        history
    }

    fn save(&self) {
        let mut data = Vec::new();
        for i in 0..self.count {
            data.extend_from_slice(&self.entries[i][..self.lens[i]]);
            data.push(b'\n');
        }
        let _ = crate::vfs::write_file(HISTORY_FILE, &data);
    }

    fn push(&mut self, command: &[u8]) -> bool {
        let command = &command[..command.len().min(MAX_LINE_LEN)];
        if command.is_empty() {
            return false;
        }
        if self.count > 0 {
            let last = self.count - 1;
            if self.lens[last] == command.len() && &self.entries[last][..self.lens[last]] == command
            {
                return false;
            }
        }

        if self.count < HISTORY_CAPACITY {
            let index = self.count;
            self.count += 1;
            self.entries[index][..command.len()].copy_from_slice(command);
            self.lens[index] = command.len();
        } else {
            for i in 0..HISTORY_CAPACITY - 1 {
                self.entries[i] = self.entries[i + 1];
                self.lens[i] = self.lens[i + 1];
            }
            let index = HISTORY_CAPACITY - 1;
            self.entries[index][..command.len()].copy_from_slice(command);
            self.lens[index] = command.len();
        }
        true
    }

    fn get(&self, index: usize) -> Option<&[u8]> {
        (index < self.count).then_some(&self.entries[index][..self.lens[index]])
    }
}

struct CompletionState {
    base: Vec<u8>,
    token_start: usize,
    candidates: Vec<String>,
    next: usize,
}

struct ReverseSearchState {
    query: Vec<u8>,
    next_index: Option<usize>,
}

pub fn run_shell() -> ! {
    let mut history = CommandHistory::load();
    let mut editor = LineEditor::new();
    let mut draft = LineEditor::new();
    let mut history_index: Option<usize> = None;
    let mut completion: Option<CompletionState> = None;
    let mut reverse_search: Option<ReverseSearchState> = None;

    print_prompt();
    let (mut prompt_x, mut prompt_y) = vga::get_cursor();

    loop {
        loop {
            if unsafe { keyboard_has_char() } {
                break;
            }
            crate::net::poll();
            if unsafe { keyboard_has_char() } {
                break;
            }
            unsafe {
                core::arch::asm!("hlt");
            }
        }

        let key = unsafe { keyboard_getchar() };
        if key != KEY_TAB {
            completion = None;
        }
        if key != KEY_CTRL_R {
            reverse_search = None;
        }
        match key {
            KEY_ENTER | KEY_RETURN => {
                set_input_cursor(prompt_x, prompt_y, editor.cursor());
                println!();
                if !editor.as_bytes().is_empty() {
                    let command = editor.as_bytes();
                    if history.push(command) {
                        history.save();
                    }
                    if let Ok(command) = core::str::from_utf8(command) {
                        commands::handle_command(command);
                    }
                }
                editor.clear();
                history_index = None;
                print_prompt();
                (prompt_x, prompt_y) = vga::get_cursor();
            }

            KEY_CTRL_C => {
                set_input_cursor(prompt_x, prompt_y, editor.cursor());
                print_colored!(Color::LightRed, Color::Black, "^C\n");
                editor.clear();
                history_index = None;
                print_prompt();
                (prompt_x, prompt_y) = vga::get_cursor();
            }

            KEY_CTRL_R => {
                reverse_search_input(
                    &mut editor,
                    &history,
                    &mut reverse_search,
                    prompt_x,
                    prompt_y,
                );
            }

            KEY_CTRL_L => {
                vga::clear_screen();
                print_prompt();
                (prompt_x, prompt_y) = vga::get_cursor();
                redraw_line(prompt_x, prompt_y, &editor, 0);
            }

            KEY_LEFT => move_input_cursor(&mut editor, prompt_x, prompt_y, LineEditor::move_left),
            KEY_RIGHT => move_input_cursor(&mut editor, prompt_x, prompt_y, LineEditor::move_right),
            KEY_SHIFT_LEFT => {
                editor.extend_left();
                redraw_line(prompt_x, prompt_y, &editor, editor.len());
            }
            KEY_SHIFT_RIGHT => {
                editor.extend_right();
                redraw_line(prompt_x, prompt_y, &editor, editor.len());
            }
            KEY_SHIFT_HOME => {
                editor.extend_home();
                redraw_line(prompt_x, prompt_y, &editor, editor.len());
            }
            KEY_SHIFT_END => {
                editor.extend_end();
                redraw_line(prompt_x, prompt_y, &editor, editor.len());
            }

            KEY_CTRL_LEFT => {
                move_input_cursor(&mut editor, prompt_x, prompt_y, LineEditor::move_word_left)
            }
            KEY_CTRL_RIGHT => {
                move_input_cursor(&mut editor, prompt_x, prompt_y, LineEditor::move_word_right)
            }
            KEY_HOME | KEY_CTRL_A => {
                move_input_cursor(&mut editor, prompt_x, prompt_y, LineEditor::move_home)
            }
            KEY_END | KEY_CTRL_E => {
                move_input_cursor(&mut editor, prompt_x, prompt_y, LineEditor::move_end)
            }

            KEY_UP => {
                if history.count > 0 {
                    let next = match history_index {
                        None => {
                            draft = editor.clone();
                            history.count - 1
                        }
                        Some(index) if index > 0 => index - 1,
                        Some(index) => index,
                    };
                    history_index = Some(next);
                    if let Some(command) = history.get(next) {
                        let old_len = editor.len();
                        editor.set_line(command);
                        redraw_line(prompt_x, prompt_y, &editor, old_len);
                    }
                }
            }
            KEY_DOWN => {
                if let Some(index) = history_index {
                    if index + 1 < history.count {
                        let next = index + 1;
                        history_index = Some(next);
                        if let Some(command) = history.get(next) {
                            let old_len = editor.len();
                            editor.set_line(command);
                            redraw_line(prompt_x, prompt_y, &editor, old_len);
                        }
                    } else {
                        history_index = None;
                        let old_len = editor.len();
                        editor = draft.clone();
                        redraw_line(prompt_x, prompt_y, &editor, old_len);
                    }
                }
            }

            KEY_BACKSPACE | KEY_DEL_CHAR => {
                let old_len = editor.len();
                editor.backspace();
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }
            KEY_DELETE | KEY_CTRL_D => {
                let old_len = editor.len();
                editor.delete_char();
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }
            KEY_ALT_BACKSPACE | KEY_CTRL_W => {
                let old_len = editor.len();
                editor.delete_word_backward();
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }
            KEY_ALT_DELETE => {
                let old_len = editor.len();
                editor.delete_word_forward();
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }
            KEY_CTRL_U => {
                let old_len = editor.len();
                editor.clear();
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }
            KEY_CTRL_K => {
                let old_len = editor.len();
                editor.kill_to_end();
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }

            KEY_SHIFT_TAB => {
                editor.clear_selection();
                redraw_line(prompt_x, prompt_y, &editor, editor.len());
            }
            KEY_TAB => {
                let old_len = editor.len();
                editor.delete_selection();
                if let Some(candidates) = complete_input(&mut editor, &mut completion) {
                    set_input_cursor(prompt_x, prompt_y, editor.len());
                    println!();
                    for candidate in candidates {
                        println!("{}", candidate);
                    }
                    print_prompt();
                    (prompt_x, prompt_y) = vga::get_cursor();
                }
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }

            ascii if ascii >= 32 && ascii <= 126 => {
                let old_len = editor.len();
                editor.insert(ascii as u8);
                redraw_line(prompt_x, prompt_y, &editor, old_len);
            }
            _ => {}
        }
    }
}

fn input_position(prompt_x: usize, prompt_y: usize, offset: usize) -> Option<(usize, usize)> {
    let (cols, rows) = vga::get_dimensions();
    if cols == 0 || rows == 0 {
        return None;
    }
    let absolute = prompt_x.saturating_add(offset);
    let x = absolute % cols;
    let y = prompt_y.saturating_add(absolute / cols);
    (y < rows).then_some((x, y))
}

fn set_input_cursor(prompt_x: usize, prompt_y: usize, offset: usize) {
    if let Some((x, y)) = input_position(prompt_x, prompt_y, offset) {
        vga::set_cursor(x, y);
    }
}

fn move_input_cursor(editor: &mut LineEditor, prompt_x: usize, prompt_y: usize, op: fn(&mut LineEditor)) {
    let had_selection = editor.selection().is_some();
    op(editor);
    if had_selection {
        redraw_line(prompt_x, prompt_y, editor, editor.len());
    } else {
        set_input_cursor(prompt_x, prompt_y, editor.cursor());
    }
}

fn draw_input_cell(prompt_x: usize, prompt_y: usize, offset: usize, ch: u8) {
    if let Some((x, y)) = input_position(prompt_x, prompt_y, offset) {
        vga::putchar_at(ch, vga::make_color(Color::White, Color::Black), x, y);
    }
}

fn redraw_line(prompt_x: usize, prompt_y: usize, editor: &LineEditor, old_len: usize) {
    let len = editor.len();
    if editor.selection().is_none() && old_len + 1 == len && editor.cursor() == len {
        draw_input_cell(prompt_x, prompt_y, len - 1, editor.as_bytes()[len - 1]);
        set_input_cursor(prompt_x, prompt_y, editor.cursor());
        return;
    }
    if editor.selection().is_none() && old_len == len + 1 && editor.cursor() == len {
        draw_input_cell(prompt_x, prompt_y, len, b' ');
        set_input_cursor(prompt_x, prompt_y, editor.cursor());
        return;
    }

    let draw_len = len.max(old_len);
    let selection = editor.selection();

    for offset in 0..draw_len {
        let Some((x, y)) = input_position(prompt_x, prompt_y, offset) else {
            break;
        };
        let ch = if offset < len {
            editor.as_bytes()[offset]
        } else {
            b' '
        };
        let selected = selection.is_some_and(|(start, end)| offset >= start && offset < end);
        let color = if selected {
            vga::make_color(Color::Black, Color::LightGray)
        } else {
            vga::make_color(Color::White, Color::Black)
        };
        vga::putchar_at(ch, color, x, y);
    }
    set_input_cursor(prompt_x, prompt_y, editor.cursor());
}

fn complete_input(
    editor: &mut LineEditor,
    state: &mut Option<CompletionState>,
) -> Option<Vec<String>> {
    if editor.cursor() != editor.len() {
        return None;
    }

    if let Some(completion) = state.as_mut() {
        let candidate = completion.candidates[completion.next].clone();
        completion.next = (completion.next + 1) % completion.candidates.len();
        replace_completion(editor, &completion.base, completion.token_start, &candidate);
        return None;
    }

    let start = editor
        .as_bytes()
        .iter()
        .rposition(|&byte| byte == b' ')
        .map_or(0, |index| index + 1);
    let prefix = &editor.as_bytes()[start..];
    let mut candidates = Vec::new();

    if start == 0 {
        for name in [
            "help", "clear", "about", "version", "sysinfo", "free", "meminfo", "uptime", "pwd",
            "cd", "mkdir", "ls", "rm", "rmdir", "cp", "mv", "stat", "cat", "touch", "write",
            "syscall", "echo", "color", "calc", "ifconfig", "netinfo", "dhcp", "dns", "nslookup",
            "ping", "arp", "netstat", "curl", "fetch", "httpd", "nc", "date", "time", "mway",
            "vmm", "panic", "reboot", "lalaufetch", "mouse", "paint", "uname", "whoami",
            "hostname", "motd", "sudo",
        ] {
            if name.as_bytes().starts_with(prefix) && name.len() > prefix.len() {
                candidates.push(String::from(name));
            }
        }
    } else {
        for (name, _) in crate::vfs::list_files() {
            if name.as_bytes().starts_with(prefix) && name.len() > prefix.len() {
                candidates.push(name);
            }
        }
    }

    match candidates.len() {
        0 => None,
        1 => {
            let base = editor.as_bytes().to_vec();
            replace_completion(editor, &base, start, &candidates[0]);
            None
        }
        _ => {
            *state = Some(CompletionState {
                base: editor.as_bytes().to_vec(),
                token_start: start,
                candidates: candidates.clone(),
                next: 0,
            });
            Some(candidates)
        }
    }
}

fn replace_completion(editor: &mut LineEditor, base: &[u8], token_start: usize, name: &str) {
    let mut line = Vec::with_capacity(token_start + name.len());
    line.extend_from_slice(&base[..token_start]);
    line.extend_from_slice(name.as_bytes());
    editor.set_line(&line);
}

fn reverse_search_input(
    editor: &mut LineEditor,
    history: &CommandHistory,
    state: &mut Option<ReverseSearchState>,
    prompt_x: usize,
    prompt_y: usize,
) {
    if state.is_none() {
        *state = Some(ReverseSearchState {
            query: editor.as_bytes().to_vec(),
            next_index: history.count.checked_sub(1),
        });
    }

    let search = state.as_mut().expect("reverse search state initialized");
    let query = &search.query;
    let mut index = search.next_index;
    while let Some(candidate_index) = index {
        index = candidate_index.checked_sub(1);
        if let Some(command) = history.get(candidate_index) {
            if query.is_empty() || command.windows(query.len()).any(|window| window == query) {
                let old_len = editor.len();
                editor.set_line(command);
                search.next_index = index;
                redraw_line(prompt_x, prompt_y, editor, old_len);
                return;
            }
        }
    }
    search.next_index = None;
}

fn print_prompt() {
    let directory = crate::vfs::current_dir();
    print_colored!(Color::LightGreen, Color::Black, "nyxara");
    print_colored!(Color::LightCyan, Color::Black, " {} > ", directory);
}