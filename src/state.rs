use raylib::prelude::Vector2;
use std::sync::mpsc::Sender;
use std::cell::RefCell;
use crate::gcode::decode_gcode;

#[derive(Clone)]
pub struct LogEntry {
    pub text: String,
    pub explanation: String,
}

pub struct PathSegment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub s: f32,
}

pub struct AppState {
    pub distance: f32,
    pub feed_rate: f32,
    pub power: f32,
    pub port: String,
    pub wattage: String,
    pub v_pos: Vector2,
    pub paths: Vec<PathSegment>,
    pub last_command: String,
    pub copied_at: Option<std::time::Instant>,
    pub serial_logs: Vec<LogEntry>,
    pub tx: Sender<String>,
}

impl AppState {
    pub fn send_command(&mut self, cmd: String) {
        let cmd = cmd.trim().to_string();
        let explanation = decode_gcode(&cmd);

        // State Update Logic for virtual view
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let mut has_g90 = false;
        let mut has_g91 = false;
        let mut has_g0 = false;
        let mut has_g1 = false;
        let is_jog = cmd.starts_with("$J=");

        let mut x_val = None;
        let mut y_val = None;
        let mut s_val = None;

        for part in &parts {
            let p = if part.starts_with("$J=") { &part[3..] } else { *part };
            if p == "G90" { has_g90 = true; }
            else if p == "G91" { has_g91 = true; }
            else if p == "G0" { has_g0 = true; }
            else if p == "G1" { has_g1 = true; }
            else if p.starts_with('X') { x_val = p[1..].parse::<f32>().ok(); }
            else if p.starts_with('Y') { y_val = p[1..].parse::<f32>().ok(); }
            else if p.starts_with('S') { s_val = p[1..].parse::<f32>().ok(); }
        }

        if is_jog {
            if has_g91 {
                if let Some(x) = x_val { self.v_pos.x = (self.v_pos.x + x).clamp(0.0, 400.0); }
                if let Some(y) = y_val { self.v_pos.y = (self.v_pos.y + y).clamp(0.0, 400.0); }
            } else if has_g90 {
                if let Some(x) = x_val { self.v_pos.x = x.clamp(0.0, 400.0); }
                if let Some(y) = y_val { self.v_pos.y = y.clamp(0.0, 400.0); }
            }
        } else if has_g0 || has_g1 {
            let old_pos = self.v_pos;
            if has_g91 {
                if let Some(x) = x_val { self.v_pos.x = (self.v_pos.x + x).clamp(0.0, 400.0); }
                if let Some(y) = y_val { self.v_pos.y = (self.v_pos.y + y).clamp(0.0, 400.0); }
            } else if has_g90 {
                if let Some(x) = x_val { self.v_pos.x = x.clamp(0.0, 400.0); }
                if let Some(y) = y_val { self.v_pos.y = y.clamp(0.0, 400.0); }
            }

            if has_g1 {
                self.paths.push(PathSegment {
                    x1: old_pos.x,
                    y1: old_pos.y,
                    x2: self.v_pos.x,
                    y2: self.v_pos.y,
                    s: s_val.unwrap_or(self.power),
                });
            }
        } else if cmd == "G92 X0 Y0" {
            self.v_pos = Vector2::new(0.0, 0.0);
        }

        self.last_command = cmd.clone();
        self.serial_logs.push(LogEntry {
            text: format!("Command: {}", cmd),
            explanation,
        });
        if self.serial_logs.len() > 100 {
            self.serial_logs.remove(0);
        }
        let _ = self.tx.send(cmd);
    }
}

pub struct StringArena {
    pub strings: RefCell<Vec<Box<str>>>,
}

impl StringArena {
    pub fn new() -> Self {
        Self { strings: RefCell::new(Vec::with_capacity(100)) }
    }

    pub fn push(&self, s: String) -> &str {
        let mut strings = self.strings.borrow_mut();
        let sanitized = s.replace('\0', "").into_boxed_str();
        let ptr = sanitized.as_ptr();
        let len = sanitized.len();
        strings.push(sanitized);
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }
    }

    pub fn clear(&self) {
        self.strings.borrow_mut().clear();
    }
}
