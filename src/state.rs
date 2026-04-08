use crate::gcode::decode_gcode;
use raylib::prelude::Vector2;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::mpsc::Sender;

fn get_ts() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let hh = (secs / 3600) % 24;
    let mm = (secs / 60) % 60;
    let ss = secs % 60;
    format!("{:02}:{:02}:{:02}", hh, mm, ss)
}

#[derive(Clone)]
pub struct LogEntry {
    pub text: String,
    pub explanation: String,
    pub is_response: bool,
    pub timestamp: String,
}

pub struct PathSegment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub s: f32,
    pub intensity: f32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum UITab {
    Manual,
    Pattern,
    Image,
    Text,
}

pub struct AppState {
    pub current_tab: UITab,
    pub distance: f32,
    pub feed_rate: f32,
    pub power: f32,
    pub passes: u32,
    pub scale: f32,
    pub log_scroll_offset: f32,
    pub col2_scroll_offset: f32,
    pub is_absolute: bool,
    pub port: String,
    pub wattage: String,
    pub v_pos: Vector2,
    pub machine_pos: Vector2,
    pub machine_state: String,
    pub paths: Vec<PathSegment>,
    pub preview_paths: Vec<PathSegment>,
    pub preview_pattern: Option<String>,
    pub custom_svg_path: Option<String>,
    pub custom_image_path: Option<String>,
    pub last_command: String,
    pub copied_at: Option<std::time::Instant>,
    pub serial_logs: VecDeque<LogEntry>,
    pub tx: Sender<String>,
    pub boundary_enabled: bool,
    pub boundary_x: f32,
    pub boundary_y: f32,
    pub boundary_w: f32,
    pub boundary_h: f32,
    pub img_low_fidelity: f32,
    pub img_high_fidelity: f32,
    pub is_processing: bool,
    pub text_content: String,
    pub text_font: String,
    pub text_is_bold: bool,
    pub text_is_outline: bool,
    pub text_letter_spacing: f32,
    pub text_line_spacing: f32,
    pub text_curve_steps: u32,
    pub text_lines_per_mm: f32,
    pub available_fonts: Vec<String>,
    pub text_font_dropdown_open: bool,
    pub text_font_scroll_offset: f32,
    pub is_text_input_active: bool,
    pub current_preview_power: f32,
}

impl AppState {
    pub fn process_command_for_preview(&mut self, cmd: &str) {
        // Handle potential multiple commands on one line or space-separated commands
        for single_cmd in cmd.split('\n') {
            let parts: Vec<&str> = single_cmd.split_whitespace().collect();
            let mut has_g0 = false;
            let mut has_g1 = false;
            let mut has_g2 = false;
            let mut has_g3 = false;

            let mut x_val = None;
            let mut y_val = None;
            let mut r_val = None;
            let mut has_m5 = false;

            for part in &parts {
                if *part == crate::gcode::CMD_ABSOLUTE_POS {
                    self.is_absolute = true;
                } else if *part == crate::gcode::CMD_RELATIVE_POS {
                    self.is_absolute = false;
                } else if *part == crate::gcode::CMD_HOME {
                    self.v_pos = raylib::prelude::Vector2 {
                        x: 0.0,
                        y: 0.0,
                    };
                } else if *part == "G0" {
                    has_g0 = true;
                } else if *part == "G1" {
                    has_g1 = true;
                } else if *part == "G2" {
                    has_g2 = true;
                } else if *part == "G3" {
                    has_g3 = true;
                } else if *part == "M3" || *part == "M4" {
                    // Check if there's an S value in the same command
                } else if *part == crate::gcode::CMD_LASER_OFF {
                    has_m5 = true;
                } else if part.starts_with('X') {
                    x_val = part[1..].parse::<f32>().ok();
                } else if part.starts_with('Y') {
                    y_val = part[1..].parse::<f32>().ok();
                } else if part.starts_with('S') {
                    if let Ok(val) = part[1..].parse::<f32>() {
                        self.current_preview_power = val;
                    }
                } else if part.starts_with('R') {
                    r_val = part[1..].parse::<f32>().ok();
                }
            }

            if has_m5 {
                self.current_preview_power = 0.0;
            }

            if has_g0 || has_g1 || has_g2 || has_g3 {
                let old_pos = self.v_pos;
                let mut target = self.v_pos;
                if self.is_absolute {
                    if let Some(x) = x_val {
                        target.x = x.clamp(0.0, 400.0);
                    }
                    if let Some(y) = y_val {
                        target.y = y.clamp(0.0, 400.0);
                    }
                } else {
                    if let Some(x) = x_val {
                        target.x = (target.x + x).clamp(0.0, 400.0);
                    }
                    if let Some(y) = y_val {
                        target.y = (target.y + y).clamp(0.0, 400.0);
                    }
                }

                if has_g1 {
                    let intensity = (self.current_preview_power / 1000.0).clamp(0.0, 1.0);
                    if intensity > 0.01 {
                        self.preview_paths.push(PathSegment {
                            x1: old_pos.x,
                            y1: old_pos.y,
                            x2: target.x,
                            y2: target.y,
                            s: self.current_preview_power,
                            intensity,
                        });
                    }
                    self.v_pos = target;
                } else if has_g0 {
                    self.v_pos = target;
                } else if (has_g2 || has_g3) && r_val.is_some() {
                    let r = r_val.unwrap();
                    let start = old_pos;
                    let end = target;
                    let dx = end.x - start.x;
                    let dy = end.y - start.y;
                    let d2 = dx * dx + dy * dy;
                    let d = d2.sqrt();
                    if d > 0.0 && d <= 2.0 * r.abs() + 0.1 {
                        let h = (r * r - d2 / 4.0).max(0.0).sqrt();
                        let mut cx = (start.x + end.x) / 2.0;
                        let mut cy = (start.y + end.y) / 2.0;
                        let multiplier = if (has_g2 && r > 0.0) || (has_g3 && r < 0.0) {
                            1.0
                        } else {
                            -1.0
                        };
                        cx += multiplier * h * dy / d;
                        cy -= multiplier * h * dx / d;
                        let start_angle = (start.y - cy).atan2(start.x - cx);
                        let mut end_angle = (end.y - cy).atan2(end.x - cx);
                        if has_g2 {
                            if end_angle >= start_angle {
                                end_angle -= 2.0 * std::f32::consts::PI;
                            }
                        } else {
                            if end_angle <= start_angle {
                                end_angle += 2.0 * std::f32::consts::PI;
                            }
                        }
                        let segments = 20;
                        let mut prev_p = start;
                        for i in 1..=segments {
                            let t = i as f32 / segments as f32;
                            let angle = start_angle + t * (end_angle - start_angle);
                            let next_p = Vector2::new(cx + r.abs() * angle.cos(), cy + r.abs() * angle.sin());
                            let intensity = (self.current_preview_power / 1000.0).clamp(0.0, 1.0);
                            self.preview_paths.push(PathSegment {
                                x1: prev_p.x,
                                y1: prev_p.y,
                                x2: next_p.x,
                                y2: next_p.y,
                                s: self.current_preview_power,
                                intensity,
                            });
                            prev_p = next_p;
                        }
                    }
                    self.v_pos = target;
                } else {
                    self.v_pos = target;
                }
            }
        }
    }

    pub fn send_command(&mut self, cmd_str: String) {
        let cmd_trimmed = cmd_str.trim().to_string();
        for line in cmd_trimmed.lines() {
            let cmd = line.trim();
            if cmd.is_empty() {
                continue;
            }
            let _ = self.tx.send(cmd.to_string());
        }
        self.last_command = cmd_trimmed.clone();
    }

    pub fn process_command_for_state(&mut self, cmd: &str, force_log: bool) {
        let explanation = decode_gcode(cmd);

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let mut has_g0 = false;
        let mut has_g1 = false;
        let mut has_g2 = false;
        let mut has_g3 = false;
        let is_jog = cmd.starts_with("$J=");

        let mut x_val = None;
        let mut y_val = None;
        let mut s_val = None;
        let mut r_val = None;

        for part in &parts {
            let p = if part.starts_with("$J=") {
                &part[3..]
            } else {
                *part
            };
            if p == crate::gcode::CMD_ABSOLUTE_POS {
                self.is_absolute = true;
            } else if p == crate::gcode::CMD_RELATIVE_POS {
                self.is_absolute = false;
            } else if p == "G0" {
                has_g0 = true;
            } else if p == "G1" {
                has_g1 = true;
            } else if p == "G2" {
                has_g2 = true;
            } else if p == "G3" {
                has_g3 = true;
            } else if p.starts_with('X') {
                x_val = p[1..].parse::<f32>().ok();
            } else if p.starts_with('Y') {
                y_val = p[1..].parse::<f32>().ok();
            } else if p.starts_with('S') {
                s_val = p[1..].parse::<f32>().ok();
            } else if p.starts_with('R') {
                r_val = p[1..].parse::<f32>().ok();
            }
        }

        if is_jog || has_g0 || has_g1 || has_g2 || has_g3 {
            let old_pos = self.v_pos;
            let mut target = self.v_pos;
            if self.is_absolute {
                if let Some(x) = x_val {
                    target.x = x.clamp(0.0, 400.0);
                }
                if let Some(y) = y_val {
                    target.y = y.clamp(0.0, 400.0);
                }
            } else {
                if let Some(x) = x_val {
                    target.x = (target.x + x).clamp(0.0, 400.0);
                }
                if let Some(y) = y_val {
                    target.y = (target.y + y).clamp(0.0, 400.0);
                }
            }

            if has_g1 {
                self.add_path_segment(old_pos.x, old_pos.y, target.x, target.y, s_val.unwrap_or(self.power));
                self.v_pos = target;
            } else if (has_g2 || has_g3) && r_val.is_some() {
                let r = r_val.unwrap();
                let start = old_pos;
                let end = target;
                let dx = end.x - start.x;
                let dy = end.y - start.y;
                let d2 = dx * dx + dy * dy;
                let d = d2.sqrt();
                if d > 0.0 && d <= 2.0 * r.abs() + 0.1 {
                    let h = (r * r - d2 / 4.0).max(0.0).sqrt();
                    let mut cx = (start.x + end.x) / 2.0;
                    let mut cy = (start.y + end.y) / 2.0;
                    let multiplier = if (has_g2 && r > 0.0) || (has_g3 && r < 0.0) {
                        1.0
                    } else {
                        -1.0
                    };
                    cx += multiplier * h * dy / d;
                    cy -= multiplier * h * dx / d;
                    let start_angle = (start.y - cy).atan2(start.x - cx);
                    let mut end_angle = (end.y - cy).atan2(end.x - cx);
                    if has_g2 {
                        if end_angle >= start_angle {
                            end_angle -= 2.0 * std::f32::consts::PI;
                        }
                    } else {
                        if end_angle <= start_angle {
                            end_angle += 2.0 * std::f32::consts::PI;
                        }
                    }
                    let segments = 20;
                    let mut prev_p = start;
                    for i in 1..=segments {
                        let t = i as f32 / segments as f32;
                        let angle = start_angle + t * (end_angle - start_angle);
                        let next_p = Vector2::new(cx + r.abs() * angle.cos(), cy + r.abs() * angle.sin());
                        self.add_path_segment(prev_p.x, prev_p.y, next_p.x, next_p.y, s_val.unwrap_or(self.power));
                        prev_p = next_p;
                    }
                }
                self.v_pos = target;
            } else {
                self.v_pos = target;
            }
        } else if cmd == crate::gcode::CMD_SET_ORIGIN || cmd == crate::gcode::CMD_HOME {
            self.v_pos = Vector2::new(0.0, 0.0);
        }

        if cmd != "?" || force_log {
            self.serial_logs.push_back(LogEntry {
                text: format!("SEND: {}", cmd),
                explanation,
                is_response: false,
                timestamp: get_ts(),
            });
            if self.serial_logs.len() > 1000 {
                self.serial_logs.pop_front();
            }
        }
    }

    fn add_path_segment(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, s: f32) {
        let intensity = (s / 1000.0).clamp(0.0, 1.0);
        self.paths.push(PathSegment {
            x1,
            y1,
            x2,
            y2,
            s,
            intensity,
        });
    }
}

pub struct StringArena {
    pub strings: RefCell<Vec<Box<str>>>,
}

impl StringArena {
    pub fn new() -> Self {
        Self {
            strings: RefCell::new(Vec::with_capacity(100)),
        }
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
