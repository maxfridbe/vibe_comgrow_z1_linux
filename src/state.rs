use std::fmt;
use std::sync::Arc;
use crate::gcode::decode_gcode;
use raylib::prelude::Vector2;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::mpsc::Sender;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bounds {
    pub enabled: bool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BurnConfig {
    pub power: f32,
    pub feed_rate: f32,
    pub scale: f32,
    pub passes: u32,
    pub bounds: Bounds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageBurnConfig {
    pub base: BurnConfig,
    pub low_fid: f32,
    pub high_fid: f32,
    pub lines_per_mm: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextBurnConfig {
    pub base: BurnConfig,
    pub content: String,
    pub font: String,
    pub is_bold: bool,
    pub is_outline: bool,
    pub letter_spacing: f32,
    pub line_spacing: f32,
    pub curve_steps: u32,
    pub lines_per_mm: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedState {
    pub timestamp: String,
    pub label: String,
    pub current_tab: UITab,
    pub text_content: String,
    pub text_font: String,
    pub text_is_bold: bool,
    pub text_is_outline: bool,
    pub text_letter_spacing: f32,
    pub text_line_spacing: f32,
    pub text_curve_steps: u32,
    pub text_lines_per_mm: f32,
    pub power: f32,
    pub feed_rate: f32,
    pub scale: f32,
    pub passes: u32,
    pub bounds: Bounds,
    pub img_low_fidelity: f32,
    pub img_high_fidelity: f32,
    pub img_lines_per_mm: f32,
    pub custom_image_path: Option<String>,
    pub custom_svg_path: Option<String>,
}

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

#[derive(Clone)]
pub struct PathSegment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub s: f32,
    pub intensity: f32,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum UITab {
    Manual,
    Pattern,
    Image,
    Text,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum MachineState {
    Idle,
    Run,
    Hold,
    Alarm,
    Door,
    Check,
    Home,
    Sleep,
    Disconnected,
    Unknown,
}

impl MachineState {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Idle" => MachineState::Idle,
            "Run" => MachineState::Run,
            "Hold" => MachineState::Hold,
            "Alarm" => MachineState::Alarm,
            "Door" => MachineState::Door,
            "Check" => MachineState::Check,
            "Home" => MachineState::Home,
            "Sleep" => MachineState::Sleep,
            _ => MachineState::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MachineState::Idle => "Idle",
            MachineState::Run => "Run",
            MachineState::Hold => "Hold",
            MachineState::Alarm => "Alarm",
            MachineState::Door => "Door",
            MachineState::Check => "Check",
            MachineState::Home => "Home",
            MachineState::Sleep => "Sleep",
            MachineState::Disconnected => "Disconnected",
            MachineState::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for MachineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
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
    pub col2_track_height: f32,
    pub col2_dragging: bool,
    pub is_absolute: bool,
    pub port: Arc<String>,
    pub wattage: Arc<String>,
    pub v_pos: Vector2,
    pub machine_pos: Vector2,
    pub machine_state: MachineState,
    pub paths: Vec<PathSegment>,
    pub preview_paths: Vec<PathSegment>,
    pub preview_pattern: Option<Arc<String>>,
    pub custom_svg_path: Option<Arc<String>>,
    pub custom_image_path: Option<Arc<String>>,
    pub last_command: String,
    pub copied_at: Option<std::time::Instant>,
    pub serial_logs: Arc<VecDeque<LogEntry>>,
    pub tx: Sender<String>,
    pub bounds: Bounds,
    pub img_low_fidelity: f32,
    pub img_high_fidelity: f32,
    pub img_lines_per_mm: f32,
    pub is_processing: bool,
    pub preview_version: u64,
    pub text_content: Arc<String>,
    pub text_font: Arc<String>,
    pub text_is_bold: bool,
    pub text_is_outline: bool,
    pub text_letter_spacing: f32,
    pub text_line_spacing: f32,
    pub text_curve_steps: u32,
    pub text_lines_per_mm: f32,
    pub available_fonts: Arc<Vec<String>>,
    pub text_font_dropdown_open: bool,
    pub text_font_scroll_offset: f32,
    pub is_text_input_active: bool,
    pub text_cursor_index: usize,
    pub current_preview_power: f32,
    pub saved_states: Arc<Vec<SavedState>>,
    pub load_dialog_open: bool,
    pub is_burning: bool,
    pub burn_log_active: bool,
    pub active_toasts: Vec<Toast>,
    pub current_theme_index: usize,
    pub zoom_size: i32,
    pub bottom_bar_height: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub current_tab: UITab,
    pub current_theme_name: String,
    pub zoom_size: i32,
    pub port: String,
    pub bottom_bar_height: f32,
}

#[derive(Clone, Debug)]
pub enum ToastType {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u32,
    pub toast_type: ToastType,
    pub message: String,
    pub remaining_seconds: f32,
    pub has_dismiss: bool,
    pub action_label: Option<String>,
    pub action_clicked: bool,
}

impl AppState {
    pub fn get_theme(&self) -> crate::theme::Theme {
        crate::theme::THEMES[self.current_theme_index % crate::theme::THEMES.len()]
    }

    pub fn clear_preview(&mut self) {
        self.preview_pattern = None;
        self.preview_paths.clear();
        self.preview_version += 1;
    }

    pub fn add_toast(&mut self, toast_type: ToastType, message: String, seconds: f32, has_dismiss: bool, action: Option<String>) {
        static mut NEXT_ID: u32 = 0;
        let id = unsafe {
            NEXT_ID += 1;
            NEXT_ID
        };
        self.active_toasts.push(Toast {
            id,
            toast_type,
            message,
            remaining_seconds: seconds,
            has_dismiss,
            action_label: action,
            action_clicked: false,
        });
    }

    pub fn get_preview_segments(
        gcode: &str,
        mut v_pos: Vector2,
        mut is_absolute: bool,
        mut current_preview_power: f32,
    ) -> (Vec<PathSegment>, Vector2, bool, f32) {
        let mut segments = Vec::new();
        // Handle potential multiple commands on one line or space-separated commands
        for single_cmd in gcode.split('\n') {
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
                    is_absolute = true;
                } else if *part == crate::gcode::CMD_RELATIVE_POS {
                    is_absolute = false;
                } else if *part == crate::gcode::CMD_HOME {
                    v_pos = raylib::prelude::Vector2 {
                        x: 0.0,
                        y: 0.0,
                    };
                } else if *part == crate::gcode::CMD_MOVE_RAPID {
                    has_g0 = true;
                } else if *part == crate::gcode::CMD_MOVE_LINEAR {
                    has_g1 = true;
                } else if *part == crate::gcode::CMD_ARC_CW {
                    has_g2 = true;
                } else if *part == crate::gcode::CMD_ARC_CCW {
                    has_g3 = true;
                } else if *part == crate::gcode::CMD_LASER_CONST || *part == crate::gcode::CMD_LASER_DYN {
                    // Check if there's an S value in the same command
                } else if *part == crate::gcode::CMD_LASER_OFF {
                    has_m5 = true;
                } else if part.starts_with('X') {
                    x_val = part[1..].parse::<f32>().ok();
                } else if part.starts_with('Y') {
                    y_val = part[1..].parse::<f32>().ok();
                } else if part.starts_with('S') {
                    if let Ok(val) = part[1..].parse::<f32>() {
                        current_preview_power = val;
                    }
                } else if part.starts_with('R') {
                    r_val = part[1..].parse::<f32>().ok();
                }
            }

            if has_m5 {
                current_preview_power = 0.0;
            }

            if has_g0 || has_g1 || has_g2 || has_g3 {
                let old_pos = v_pos;
                let mut target = v_pos;
                if is_absolute {
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
                    let intensity = (current_preview_power / 1000.0).clamp(0.0, 1.0);
                    if intensity > 0.01 {
                        segments.push(PathSegment {
                            x1: old_pos.x,
                            y1: old_pos.y,
                            x2: target.x,
                            y2: target.y,
                            s: current_preview_power,
                            intensity,
                        });
                    }
                    v_pos = target;
                } else if has_g0 {
                    v_pos = target;
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
                        let segments_count = 20;
                        let mut prev_p = start;
                        for i in 1..=segments_count {
                            let t = i as f32 / segments_count as f32;
                            let angle = start_angle + t * (end_angle - start_angle);
                            let next_p = Vector2::new(cx + r.abs() * angle.cos(), cy + r.abs() * angle.sin());
                            let intensity = (current_preview_power / 1000.0).clamp(0.0, 1.0);
                            segments.push(PathSegment {
                                x1: prev_p.x,
                                y1: prev_p.y,
                                x2: next_p.x,
                                y2: next_p.y,
                                s: current_preview_power,
                                intensity,
                            });
                            prev_p = next_p;
                        }
                    }
                    v_pos = target;
                } else {
                    v_pos = target;
                }
            }
        }
        (segments, v_pos, is_absolute, current_preview_power)
    }

    pub fn process_command_for_preview(&mut self, cmd: &str) {
        let (segments, v_pos, is_absolute, power) = Self::get_preview_segments(
            cmd,
            self.v_pos,
            self.is_absolute,
            self.current_preview_power,
        );
        if !segments.is_empty() {
            self.preview_version += 1;
        }
        self.preview_paths.extend(segments);
        self.v_pos = v_pos;
        self.is_absolute = is_absolute;
        self.current_preview_power = power;
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
            let mut logs = (*self.serial_logs).clone();
            logs.push_back(LogEntry {
                text: format!("SEND: {}", cmd),
                explanation,
                is_response: false,
                timestamp: get_ts(),
            });
            if logs.len() > 1000 {
                logs.pop_front();
            }
            self.serial_logs = Arc::new(logs);
        }
    }

    pub fn get_burn_config(&self) -> BurnConfig {
        BurnConfig {
            power: self.power,
            feed_rate: self.feed_rate,
            scale: self.scale,
            passes: self.passes,
            bounds: self.bounds.clone(),
        }
    }

    pub fn get_image_burn_config(&self) -> ImageBurnConfig {
        ImageBurnConfig {
            base: self.get_burn_config(),
            low_fid: self.img_low_fidelity,
            high_fid: self.img_high_fidelity,
            lines_per_mm: self.img_lines_per_mm,
        }
    }

    pub fn get_text_burn_config(&self) -> TextBurnConfig {
        TextBurnConfig {
            base: self.get_burn_config(),
            content: (*self.text_content).clone(),
            font: (*self.text_font).clone(),
            is_bold: self.text_is_bold,
            is_outline: self.text_is_outline,
            letter_spacing: self.text_letter_spacing,
            line_spacing: self.text_line_spacing,
            curve_steps: self.text_curve_steps,
            lines_per_mm: self.text_lines_per_mm,
        }
    }

    pub fn capture_state(&self, label: &str) -> SavedState {
        println!("[{}] Capturing state: {}", get_ts(), label);
        SavedState {
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            label: label.to_string(),
            current_tab: self.current_tab,
            text_content: (*self.text_content).clone(),
            text_font: (*self.text_font).clone(),
            text_is_bold: self.text_is_bold,
            text_is_outline: self.text_is_outline,
            text_letter_spacing: self.text_letter_spacing,
            text_line_spacing: self.text_line_spacing,
            text_curve_steps: self.text_curve_steps,
            text_lines_per_mm: self.text_lines_per_mm,
            power: self.power,
            feed_rate: self.feed_rate,
            scale: self.scale,
            passes: self.passes,
            bounds: self.bounds.clone(),
            img_low_fidelity: self.img_low_fidelity,
            img_high_fidelity: self.img_high_fidelity,
            img_lines_per_mm: self.img_lines_per_mm,
            custom_image_path: self.custom_image_path.as_ref().map(|p| (**p).clone()),
            custom_svg_path: self.custom_svg_path.as_ref().map(|p| (**p).clone()),
        }
    }

    pub fn apply_state(&mut self, state: &SavedState) {
        println!("[{}] Applying saved state: {}", get_ts(), state.label);
        self.current_tab = state.current_tab;
        self.text_content = Arc::new(state.text_content.clone());
        self.text_font = Arc::new(state.text_font.clone());
        self.text_is_bold = state.text_is_bold;
        self.text_is_outline = state.text_is_outline;
        self.text_letter_spacing = state.text_letter_spacing;
        self.text_line_spacing = state.text_line_spacing;
        self.text_curve_steps = state.text_curve_steps;
        self.text_lines_per_mm = state.text_lines_per_mm;
        self.power = state.power;
        self.feed_rate = state.feed_rate;
        self.scale = state.scale;
        self.passes = state.passes;
        self.bounds = state.bounds.clone();
        self.img_low_fidelity = state.img_low_fidelity;
        self.img_high_fidelity = state.img_high_fidelity;
        self.img_lines_per_mm = state.img_lines_per_mm;
        self.custom_image_path = state.custom_image_path.as_ref().map(|p| Arc::new(p.clone()));
        self.custom_svg_path = state.custom_svg_path.as_ref().map(|p| Arc::new(p.clone()));
        
        // Clear preview when state is changed to avoid showing old data
        self.preview_pattern = None;
        self.preview_paths.clear();
        self.preview_version += 1;
    }

    pub fn save_persistence(&self) {
        if let Ok(path) = self.get_config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&self.saved_states) {
                let _ = std::fs::write(path, json);
            }
        }
    }

    pub fn load_persistence(&mut self) {
        if let Ok(path) = self.get_config_path() {
            if let Ok(json) = std::fs::read_to_string(path) {
                if let Ok(states) = serde_json::from_str(&json) {
                    self.saved_states = states;
                }
            }
        }
    }

    pub fn save_user_config(&self) {
        if let Ok(path) = self.get_user_config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let config = UserConfig {
                current_tab: self.current_tab,
                current_theme_name: self.get_theme().name.to_string(),
                zoom_size: self.zoom_size,
                port: (*self.port).clone(),
                bottom_bar_height: self.bottom_bar_height,
            };
            if let Ok(yaml) = serde_yaml::to_string(&config) {
                let _ = std::fs::write(path, yaml);
            }
        }
    }

    pub fn load_user_config(&mut self) {
        if let Ok(path) = self.get_user_config_path() {
            if let Ok(yaml) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_yaml::from_str::<UserConfig>(&yaml) {
                    self.current_tab = config.current_tab;
                    // Find theme index by name
                    if let Some(idx) = crate::theme::THEMES.iter().position(|t| t.name == config.current_theme_name) {
                        self.current_theme_index = idx;
                    }
                    self.zoom_size = config.zoom_size;
                    self.port = Arc::new(config.port);
                    self.bottom_bar_height = if config.bottom_bar_height > 0.0 { config.bottom_bar_height } else { 140.0 };
                }
            }
        }
    }

    fn get_config_path(&self) -> Result<std::path::PathBuf, crate::error::TrogdorError> {
        let home = std::env::var("HOME")?;
        Ok(std::path::PathBuf::from(home)
            .join(".config")
            .join("trogdor")
            .join("saved_states.json"))
    }

    fn get_user_config_path(&self) -> Result<std::path::PathBuf, crate::error::TrogdorError> {
        let home = std::env::var("HOME")?;
        Ok(std::path::PathBuf::from(home)
            .join(".config")
            .join("trogdor")
            .join("config.yml"))
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
