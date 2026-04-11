#![windows_subsystem = "windows"]

mod comm;
mod error;
mod gcode;
mod icons;
mod state;
mod styles;
mod theme;
mod svg_helper;
mod ui_components;
mod ui_image;
mod ui_manual;
mod ui_svg;
mod ui_test;
mod ui_text;

use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::math::{Dimensions, Vector2 as ClayVector2};
use clay_layout::render_commands::RenderCommandConfig;
use clay_layout::{Clay, Declaration, fixed, grow};
use font_kit::source::SystemSource;
use raylib::prelude::*;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use styles::*;

mod cli_and_helpers;
mod virtual_device;

use crate::cli_and_helpers::*;
use crate::comm::start_serial_thread;
use crate::icons::*;
use crate::state::{AppState, MachineState, StringArena, UITab};
use crate::ui_components::{Command, Section, render_tab_btn};

const FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

trait FontMeasureEx {
    fn measure_text_ex(&self, text: &str, size: f32, spacing: f32) -> raylib::math::Vector2;
}

impl FontMeasureEx for raylib::prelude::Font {
    fn measure_text_ex(&self, text: &str, size: f32, spacing: f32) -> raylib::math::Vector2 {
        self.measure_text(text, size, spacing)
    }
}

fn main() -> Result<(), crate::error::TrogdorError> {
    let _args: Vec<String> = std::env::args().collect();

    let state = Arc::new(Mutex::new(AppState {
        current_tab: UITab::Manual,
        distance: 10.0,
        feed_rate: 3000.0,
        power: 100.0,
        passes: 1,
        scale: 1.0,
        log_scroll_offset: 0.0,
        col2_scroll_offset: 0.0,
        col2_track_height: 0.0,
        col2_dragging: false,
        is_absolute: true,
        port: Arc::new("/dev/ttyUSB0".to_string()),
        wattage: Arc::new("10W".to_string()),
        v_pos: Vector2::new(0.0, 0.0),
        machine_pos: Vector2::new(0.0, 0.0),
        machine_state: MachineState::Idle,
        paths: Vec::new(),
        preview_paths: Vec::new(),
        preview_pattern: None,
        custom_svg_path: None,
        custom_image_path: None,
        last_command: String::new(),
        copied_at: None,
        serial_logs: Arc::new(std::collections::VecDeque::new()),
        tx: mpsc::channel().0,
        bounds: crate::state::Bounds {
            enabled: false,
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 400.0,
        },
        img_low_fidelity: 0.0,
        img_high_fidelity: 1.0,
        img_lines_per_mm: 5.0,
        is_processing: false,
        preview_version: 0,
        text_content: Arc::new("Comgrow Z1".to_string()),
        text_font: Arc::new("Default".to_string()),
        text_is_bold: false,
        text_is_outline: false,
        text_letter_spacing: 0.0,
        text_line_spacing: 1.0,
        text_curve_steps: 10,
        text_lines_per_mm: 5.0,
        available_fonts: Arc::new({
            let mut fonts = SystemSource::new().all_families().unwrap_or_default();
            fonts.sort();
            fonts
        }),
        text_font_dropdown_open: false,
        text_font_scroll_offset: 0.0,
        is_text_input_active: false,
        text_cursor_index: 0,
        current_preview_power: 1000.0,
        saved_states: Arc::new(Vec::new()),
        load_dialog_open: false,
        is_burning: false,
        burn_log_active: false,
        active_toasts: Vec::new(),
        current_theme_index: 0,
        zoom_size: 64,
        bottom_bar_height: 140.0,
    }));

    {
        let mut g = state.lock().unwrap();
        g.load_persistence();
        g.load_user_config();
    }

    let (tx, rx) = mpsc::channel();
    state.lock().unwrap().tx = tx;
    start_serial_thread(Arc::clone(&state), rx);

    let (mut rl, thread) =
        raylib::init().size(1280, 800).title("Comgrow Z1 Laser GRBL Runner").vsync().resizable().build();

    rl.set_exit_key(None);
    rl.set_target_fps(60);

    let mut chars: Vec<char> = (32..127).map(|c| c as u8 as char).collect();
    let icons_list: &[&str] = &[
        ICON_TERMINAL,
        ICON_MOVE,
        ICON_POWER,
        ICON_HOME,
        ICON_UNLOCK,
        ICON_REFRESH,
        ICON_SETTINGS,
        ICON_LAYERS,
        ICON_GAUGE,
        ICON_LASER,
        ICON_ARROW_UP,
        ICON_ARROW_DOWN,
        ICON_ARROW_LEFT,
        ICON_ARROW_RIGHT,
        ICON_CROSSHAIR,
        ICON_FLAME,
        ICON_USB,
        ICON_SHIELD,
        ICON_CPU,
        ICON_TRASH,
        ICON_COPY,
        ICON_SWEEP,
        ICON_SERIAL,
        ICON_CHECK,
        ICON_FILE,
        ICON_EYE,
        ICON_EYE_SLASH,
        ICON_STOP,
        ICON_FONT,
        ICON_SQUARE_VECTOR,
        ICON_SPINNER,
        ICON_IMAGE,
    ];
    for &icon in icons_list {
        for c in icon.chars() {
            if !chars.contains(&c) {
                chars.push(c);
            }
        }
    }

    let font_chars: String = chars.iter().collect();

    let mut clay = Clay::new(Dimensions::new(1280.0, 800.0));
    clay.set_measure_text_function(|text, config| {
        let size = config.font_size as f32;
        unsafe {
            if !ui_components::MEASURE_FONT_PTR.is_null() {
                let f = &*ui_components::MEASURE_FONT_PTR;
                let m = f.measure_text_ex(text, size, 0.0);
                Dimensions::new(m.x, m.y)
            } else {
                let width = text.len() as f32 * (size * 0.5);
                Dimensions::new(width, size)
            }
        }
    });
    let arena = StringArena::new();
    let mut clipboard = Clipboard::new().ok();
    let initial_zoom = state.lock().unwrap().zoom_size;
    let mut font = rl
        .load_font_from_memory(&thread, ".ttf", FONT_DATA, initial_zoom, Some(&font_chars))
        .expect("Failed to load font");

    unsafe {
        ui_components::MEASURE_FONT_PTR = &font as *const Font;
    }

    let mut sections = vec![
        Section {
            title: "Real-Time & System",
            icon: ICON_REFRESH,
            color: COLOR_USB_ICON,
            commands: vec![
                Command {
                    label: "Status",
                    cmd: gcode::CMD_STATUS_REPORT,
                },
                Command {
                    label: "Home",
                    cmd: gcode::CMD_HOME,
                },
                Command {
                    label: "Settings",
                    cmd: gcode::CMD_SETTINGS_REPORT,
                },
                Command {
                    label: "Hold",
                    cmd: gcode::CMD_FEED_HOLD,
                },
                Command {
                    label: "Resume",
                    cmd: gcode::CMD_CYCLE_START,
                },
                Command {
                    label: "Unlock",
                    cmd: gcode::CMD_UNLOCK,
                },
                Command {
                    label: "Reset",
                    cmd: gcode::CMD_SOFT_RESET,
                },
            ],
        },
        Section {
            title: "Laser & Air",
            icon: ICON_FLAME,
            color: COLOR_WARNING,
            commands: vec![
                Command {
                    label: "Dynamic",
                    cmd: gcode::CMD_LASER_DYN,
                },
                Command {
                    label: "Air On",
                    cmd: gcode::CMD_AIR_ASSIST_ON,
                },
                Command {
                    label: "Air Off",
                    cmd: gcode::CMD_AIR_ASSIST_OFF,
                },
            ],
        },
        Section {
            title: "Calibration",
            icon: ICON_GAUGE,
            color: COLOR_SUCCESS_LIGHT,
            commands: vec![
                Command {
                    label: "Max S",
                    cmd: gcode::SET_MAX_S_1000,
                },
                Command {
                    label: "Laser Mode",
                    cmd: gcode::SET_LASER_MODE_1,
                },
                Command {
                    label: "Y-Steps",
                    cmd: gcode::SET_Y_STEPS_80,
                },
                Command {
                    label: "Rotary",
                    cmd: gcode::SET_Y_STEPS_65,
                },
                Command {
                    label: "X-Steps",
                    cmd: gcode::SET_X_STEPS_80,
                },
            ],
        },
        Section {
            title: "Safety",
            icon: ICON_SHIELD,
            color: COLOR_SLIDER_POWER,
            commands: vec![
                Command {
                    label: "Gyro",
                    cmd: gcode::SET_GYRO_16,
                },
                Command {
                    label: "Hard Lmt",
                    cmd: gcode::SET_HARD_LIMITS_1,
                },
                Command {
                    label: "Soft Lmt",
                    cmd: gcode::SET_SOFT_LIMITS_1,
                },
                Command {
                    label: "X-Travel",
                    cmd: gcode::SET_X_MAX_TRAVEL_400,
                },
                Command {
                    label: "Y-Travel",
                    cmd: gcode::SET_Y_MAX_TRAVEL_400,
                },
            ],
        },
        Section {
            title: "Modals",
            icon: ICON_LAYERS,
            color: COLOR_ACCENT_PURPLE_VIRTUAL,
            commands: vec![
                Command {
                    label: "Abs",
                    cmd: gcode::CMD_ABSOLUTE_POS,
                },
                Command {
                    label: "Inc",
                    cmd: gcode::CMD_RELATIVE_POS,
                },
                Command {
                    label: "mm",
                    cmd: gcode::CMD_MILLIMETERS,
                },
                Command {
                    label: "inch",
                    cmd: gcode::CMD_INCHES,
                },
            ],
        },
        Section {
            title: "Test Patterns",
            icon: ICON_GAUGE,
            color: COLOR_PINK,
            commands: vec![
                Command {
                    label: "Square",
                    cmd: "",
                },
                Command {
                    label: "Heart",
                    cmd: "",
                },
                Command {
                    label: "Star",
                    cmd: "",
                },
                Command {
                    label: "Car",
                    cmd: "",
                },
                Command {
                    label: "Stars8",
                    cmd: "",
                },
                Command {
                    label: "Stars9",
                    cmd: "",
                },
            ],
        },
    ];

    let mut pre_fullscreen_size = raylib::math::Vector2::new(1280.0, 800.0);

    let mut preview_texture = rl.load_render_texture(&thread, 2000, 2000).expect("Failed to create render texture");
    preview_texture.set_texture_filter(&thread, raylib::prelude::TextureFilter::TEXTURE_FILTER_BILINEAR);
    let mut last_preview_version = 0u64;

    while !rl.window_should_close() {
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            let curr = rl.is_window_fullscreen();
            if curr {
                rl.toggle_fullscreen();
                rl.set_window_size(pre_fullscreen_size.x as i32, pre_fullscreen_size.y as i32);
            } else {
                pre_fullscreen_size =
                    raylib::math::Vector2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
                let m = raylib::prelude::get_current_monitor();
                rl.set_window_size(raylib::prelude::get_monitor_width(m), raylib::prelude::get_monitor_height(m));
                rl.toggle_fullscreen();
            }
        }
        arena.clear();

        if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL) {
            if rl.is_key_pressed(KeyboardKey::KEY_EQUAL) {
                let mut g = state.lock().unwrap();
                g.zoom_size = (g.zoom_size + 16).min(128);
                let new_zoom = g.zoom_size;
                g.save_user_config();
                drop(g);
                
                font = rl
                    .load_font_from_memory(&thread, ".ttf", FONT_DATA, new_zoom, Some(&font_chars))
                    .expect("Failed to load font");
                unsafe {
                    ui_components::MEASURE_FONT_PTR = &font as *const Font;
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_MINUS) {
                let mut g = state.lock().unwrap();
                g.zoom_size = (g.zoom_size - 16).max(32);
                let new_zoom = g.zoom_size;
                g.save_user_config();
                drop(g);

                font = rl
                    .load_font_from_memory(&thread, ".ttf", FONT_DATA, new_zoom, Some(&font_chars))
                    .expect("Failed to load font");
                unsafe {
                    ui_components::MEASURE_FONT_PTR = &font as *const Font;
                }
            }
        }

        let (font_scale, theme) = {
            let g = state.lock().unwrap();
            (g.zoom_size as f32 / 64.0, g.get_theme())
        };

        let mouse_pos = rl.get_mouse_position();
        let frame_time_total = rl.get_time() as f32;

        let mouse_down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let mouse_pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);

        if rl.is_key_pressed(KeyboardKey::KEY_T) && (rl.is_key_down(KeyboardKey::KEY_LEFT_ALT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_ALT)) {
            let mut guard = state.lock().unwrap();
            if !guard.is_text_input_active {
                guard.current_theme_index = (guard.current_theme_index + 1) % crate::theme::THEMES.len();
                let theme_name = crate::theme::THEMES[guard.current_theme_index].name;
                guard.add_toast(crate::state::ToastType::Info, format!("Theme: {}", theme_name), 1.5, false, None);
                guard.save_user_config();
            }
        }

        let mut scroll_delta = rl.get_mouse_wheel_move_v();

        // Handle text input
        {
            let mut g = state.lock().unwrap();
            if g.is_text_input_active {
                if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                    g.is_text_input_active = false;
                }
                
                let mut content = (*g.text_content).clone();
                let mut cursor = g.text_cursor_index;
                // Ensure cursor is at a valid char boundary
                if !content.is_char_boundary(cursor) {
                    cursor = content.len();
                }

                // Mouse click to position cursor
                if mouse_pressed {
                    let input_id = unsafe { clay_layout::id::Id { id: clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from("text_input_global"), 0, 0) } };
                    let data = unsafe { clay_layout::bindings::Clay_GetElementData(input_id.id) };
                    if data.found {
                        let rect = data.boundingBox;
                        if mouse_pos.x >= rect.x && mouse_pos.x <= rect.x + rect.width &&
                           mouse_pos.y >= rect.y && mouse_pos.y <= rect.y + rect.height {
                            // Hit test characters
                            let local_x = mouse_pos.x - (rect.x + 12.0); // 12.0 is padding
                            let local_y = mouse_pos.y - (rect.y + 12.0);
                            let font_size = 16.0 * font_scale;
                            
                            let lines: Vec<&str> = content.split('\n').collect();
                            // Line index based on Y position, clamped to available lines
                            let line_idx = (local_y / font_size).floor().max(0.0) as usize;
                            
                            if line_idx < lines.get(line_idx).map_or(0, |_| line_idx + 1) {
                                let line = lines[line_idx];
                                let mut best_cursor = 0;
                                let mut min_dist = local_x.abs();
                                
                                unsafe {
                                    if !ui_components::MEASURE_FONT_PTR.is_null() {
                                        let f = &*ui_components::MEASURE_FONT_PTR;
                                        for i in 1..=line.len() {
                                            if line.is_char_boundary(i) {
                                                let m = f.measure_text_ex(&line[..i], font_size, 0.0);
                                                let dist = (m.x - local_x).abs();
                                                if dist < min_dist {
                                                    min_dist = dist;
                                                    best_cursor = i;
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                let mut byte_offset = 0;
                                for i in 0..line_idx {
                                    byte_offset += lines[i].len() + 1;
                                }
                                cursor = byte_offset + best_cursor;
                            } else {
                                // Clicked below the last line, move to end of text
                                cursor = content.len();
                            }
                        } else {
                            // Clicked outside the box
                            g.is_text_input_active = false;
                        }
                    }
                }

                // Arrow keys
                if rl.is_key_pressed(KeyboardKey::KEY_LEFT) && cursor > 0 {
                    let mut prev = cursor - 1;
                    while prev > 0 && !content.is_char_boundary(prev) {
                        prev -= 1;
                    }
                    cursor = prev;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) && cursor < content.len() {
                    let mut next = cursor + 1;
                    while next < content.len() && !content.is_char_boundary(next) {
                        next += 1;
                    }
                    cursor = next;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_UP) {
                    let start_of_current_line = content[..cursor].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
                    let column = content[start_of_current_line..cursor].chars().count();
                    if start_of_current_line > 0 {
                        let start_of_prev_line = content[..start_of_current_line - 1].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
                        let prev_line = &content[start_of_prev_line..start_of_current_line - 1];
                        let mut new_col_bytes = 0;
                        for (i, c) in prev_line.chars().enumerate() {
                            if i >= column { break; }
                            new_col_bytes += c.len_utf8();
                        }
                        cursor = start_of_prev_line + new_col_bytes;
                    }
                }
                if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
                    let start_of_current_line = content[..cursor].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
                    let column = content[start_of_current_line..cursor].chars().count();
                    if let Some(next_newline) = content[cursor..].find('\n') {
                        let start_of_next_line = cursor + next_newline + 1;
                        let end_of_next_line = content[start_of_next_line..].find('\n').map(|idx| start_of_next_line + idx).unwrap_or(content.len());
                        let next_line = &content[start_of_next_line..end_of_next_line];
                        let mut new_col_bytes = 0;
                        for (i, c) in next_line.chars().enumerate() {
                            if i >= column { break; }
                            new_col_bytes += c.len_utf8();
                        }
                        cursor = start_of_next_line + new_col_bytes;
                    }
                }
                if rl.is_key_pressed(KeyboardKey::KEY_HOME) {
                    if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL) {
                        cursor = 0;
                    } else {
                        cursor = content[..cursor].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
                    }
                }
                if rl.is_key_pressed(KeyboardKey::KEY_END) {
                    if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL) {
                        cursor = content.len();
                    } else {
                        cursor = content[cursor..].find('\n').map(|idx| cursor + idx).unwrap_or(content.len());
                    }
                }

                // Newline
                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    content.insert(cursor, '\n');
                    cursor += 1;
                }

                // Backspace
                if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) && cursor > 0 {
                    let mut prev = cursor - 1;
                    while prev > 0 && !content.is_char_boundary(prev) {
                        prev -= 1;
                    }
                    content.remove(prev);
                    cursor = prev;
                }
                // Delete
                if rl.is_key_pressed(KeyboardKey::KEY_DELETE) && cursor < content.len() {
                    content.remove(cursor);
                }

                // Character input
                while let Some(c) = rl.get_char_pressed() {
                    content.insert(cursor, c);
                    cursor += c.len_utf8();
                }

                g.text_content = Arc::new(content);
                g.text_cursor_index = cursor;

                // Stop scroll from bubbling when typing
                scroll_delta.y = 0.0;
            }
        }

        let render_width = rl.get_render_width() as f32;
        let render_height = rl.get_render_height() as f32;
        let mut canvas_rect_actual = raylib::math::Rectangle::new(0.0, 0.0, 0.0, 0.0);

        {
            let current_version = state.lock().unwrap().preview_version;
            if current_version != last_preview_version {
                let mut td = rl.begin_texture_mode(&thread, &mut preview_texture);
                td.clear_background(raylib::color::Color::BLANK);
                let guard = state.lock().unwrap();
                let scale = 2000.0 / 400.0;
                let preview_thickness = (2000.0 / (400.0 * guard.text_lines_per_mm)).max(6.0);
                for p in &guard.preview_paths {
                    // Draw Y-down in the texture (standard coordinate space)
                    // We will flip the entire texture once during draw_texture_pro
                    let start = raylib::math::Vector2::new(
                        p.x1 * scale,
                        2000.0 - p.y1 * scale,
                    );
                    let end = raylib::math::Vector2::new(
                        p.x2 * scale,
                        2000.0 - p.y2 * scale,
                    );
                    // Boost visibility for preview by using a higher base alpha
                    let intensity = (p.intensity * 0.4 + 0.6).clamp(0.0, 1.0);
                    td.draw_line_ex(start, end, preview_thickness, raylib::color::Color::new(0, 255, 0, (intensity * 255.0) as u8));
                }
                last_preview_version = current_version;
            }
        }

        clay.set_layout_dimensions(Dimensions::new(render_width, render_height));
        clay.pointer_state(
            clay_layout::math::Vector2 {
                x: mouse_pos.x,
                y: mouse_pos.y,
            },
            mouse_down,
        );
        clay.update_scroll_containers(
            true,
            clay_layout::math::Vector2 {
                x: scroll_delta.x * 50.0,
                y: scroll_delta.y * 50.0,
            },
            rl.get_frame_time(),
        );

        {
            let mut guard = state.lock().unwrap();
            let dt = rl.get_frame_time();
            guard.active_toasts.retain_mut(|t| {
                t.remaining_seconds -= dt;
                t.remaining_seconds > 0.0
            });
        }

        let mut clay_scope = clay.begin::<Texture2D, ()>();

        // 1. MAIN APP LAYER
        let mut main_app_decl = Declaration::<Texture2D, ()>::new();
        main_app_decl
            .id(clay_scope.id("main_app_root"))
            .layout()
            .width(fixed!(render_width))
            .height(fixed!(render_height))
            .padding(Padding::all(0))
            .child_gap(0)
            .direction(LayoutDirection::TopToBottom)
            .end()
            .background_color(theme.cl_bg_main);

        clay_scope.with(&main_app_decl, |clay_scope| {
            let mut main_container = Declaration::<Texture2D, ()>::new();
            main_container
                .layout()
                .width(fixed!(render_width))
                .height(fixed!(render_height))
                .direction(LayoutDirection::TopToBottom)
                .end();

            clay_scope.with(&main_container, |clay_scope| {
                let bottom_bar_height = state.lock().unwrap().bottom_bar_height * font_scale;
                let standard_margin = (20.0 * font_scale) as u16;

            // Combined Header and Tab Bar Container
            let mut combined_header = Declaration::<Texture2D, ()>::new();
            combined_header
                .layout()
                .width(grow!())
                .direction(LayoutDirection::TopToBottom)
                .end()
                .background_color(theme.cl_bg_section);

            clay_scope.with(&combined_header, |clay_scope| {
                let mut header_decl = Declaration::<Texture2D, ()>::new();
                header_decl
                    .layout()
                    .width(grow!())
                    .height(fixed!(40.0 * font_scale))
                    .padding(Padding::all(6))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                    .end();

                clay_scope.with(&header_decl, |clay_scope| {
                let mut title_group = Declaration::<Texture2D, ()>::new();
                title_group
                    .layout()
                    .child_gap(16)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                    .end();
                clay_scope.with(&title_group, |clay_scope| {
                    let mut icon_box = Declaration::<Texture2D, ()>::new();
                    icon_box
                        .layout()
                        .padding(Padding::all(4))
                        .end()
                        .background_color(theme.cl_primary_hover)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&icon_box, |clay_scope| {
                        clay_scope.text(
                            ICON_LASER,
                            clay_layout::text::TextConfig::new()
                                .font_size((16.0 * font_scale) as u16)
                                .color(theme.cl_text_main)
                                .end(),
                        );
                    });
                    clay_scope.text(
                        "TROGDOR",
                        clay_layout::text::TextConfig::new()
                            .font_size((16.0 * font_scale) as u16)
                            .color(theme.cl_text_main)
                            .end(),
                    );
                });

                let mut spacer = Declaration::<Texture2D, ()>::new();
                spacer.layout().width(grow!()).end();
                clay_scope.with(&spacer, |_| {});

                let mut settings_group = Declaration::<Texture2D, ()>::new();
                settings_group
                    .layout()
                    .child_gap(8)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Center))
                    .end();
                clay_scope.with(&settings_group, |clay_scope| {
                    let port_h_id = clay_scope.id("port_header");
                    let (port, wattage) = {
                        let g = state.lock().unwrap();
                        (g.port.clone(), g.wattage.clone())
                    };
                    let mut port_bg = theme.cl_bg_dark;
                    let mut port_text_color = theme.cl_text_sub;
                    if clay_scope.pointer_over(port_h_id) {
                        port_bg = theme.cl_primary_hover;
                        port_text_color = theme.cl_text_main;
                        if mouse_pressed {
                            let mut g = state.lock().unwrap();
                            if *g.port == "VIRTUAL" {
                                g.port = Arc::new("/dev/ttyUSB0".to_string());
                            } else {
                                g.port = Arc::new("VIRTUAL".to_string());
                                g.machine_state = MachineState::Idle;
                                g.machine_pos = Vector2::new(0.0, 0.0);
                            }
                            g.save_user_config();
                        }
                    }
                    if *port == "VIRTUAL" {
                        port_bg = theme.cl_accent;
                        port_text_color = theme.cl_text_main;
                    }

                    let mut input_box = Declaration::<Texture2D, ()>::new();
                    input_box
                        .id(port_h_id)
                        .layout()
                        .padding(Padding::all(4))
                        .child_gap(8)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                        .end()
                        .background_color(port_bg)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&input_box, |clay_scope| {
                        clay_scope.text(
                            arena.push(format!("{}   {}", ICON_USB, port)),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(port_text_color)
                                .end(),
                        );
                    });
                    let mut wattage_box = Declaration::<Texture2D, ()>::new();
                    wattage_box
                        .layout()
                        .padding(Padding::all(4))
                        .child_gap(8)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                        .end()
                        .background_color(theme.cl_bg_dark)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&wattage_box, |clay_scope| {
                        clay_scope.text(
                            arena.push(format!("{}   {}", ICON_POWER, wattage)),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(theme.cl_text_sub)
                                .end(),
                        );
                    });

                    let estop_h_id = clay_scope.id("estop_header");
                    let mstate = state.lock().unwrap().machine_state;
                    let is_emergency = mstate == MachineState::Alarm || mstate == MachineState::Hold;
                    let mut estop_h_color = if is_emergency {
                        theme.cl_success
                    } else {
                        theme.cl_danger
                    };
                    if clay_scope.pointer_over(estop_h_id) {
                        estop_h_color = if is_emergency {
                            COLOR_SUCCESS_LIGHT
                        } else {
                            COLOR_DANGER_HOVER
                        };
                        if mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            guard.send_command(crate::gcode::CMD_FEED_HOLD.to_string());
                            guard.send_command(crate::gcode::CMD_LASER_OFF.to_string());
                            guard.send_command(crate::gcode::CMD_SOFT_RESET.to_string());
                            guard.paths.clear();
                        }
                    }
                    let mut estop_h_btn = Declaration::<Texture2D, ()>::new();
                    estop_h_btn
                        .id(estop_h_id)
                        .layout()
                        .padding(Padding::new(8, 8, 4, 4))
                        .end()
                        .background_color(estop_h_color)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&estop_h_btn, |clay_scope| {
                        clay_scope.text(
                            arena.push(format!("{}   E-STOP", ICON_STOP)),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(theme.cl_text_main)
                                .end(),
                        );
                    });
                });
            });

            let mut header_row = Declaration::<Texture2D, ()>::new();
            header_row
                .layout()
                .width(grow!())
                .direction(LayoutDirection::LeftToRight)
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Bottom))
                .end();

            clay_scope.with(&header_row, |clay_scope| {
                let mut tab_bar = Declaration::<Texture2D, ()>::new();
                tab_bar.layout().direction(LayoutDirection::LeftToRight).child_gap(0).end();
                clay_scope.with(&tab_bar, |clay_scope| {
                    let current_tab = state.lock().unwrap().current_tab;
                    if render_tab_btn(clay_scope, "tab_manual", "Manual", current_tab == UITab::Manual, &arena, font_scale, &theme) {
                        let mut g = state.lock().unwrap();
                        g.current_tab = UITab::Manual;
                        g.save_user_config();
                    }
                    if render_tab_btn(clay_scope, "tab_pattern", "Pattern", current_tab == UITab::Pattern, &arena, font_scale, &theme) {
                        let mut g = state.lock().unwrap();
                        g.current_tab = UITab::Pattern;
                        g.save_user_config();
                    }
                    if render_tab_btn(clay_scope, "tab_image", "Image", current_tab == UITab::Image, &arena, font_scale, &theme) {
                        let mut g = state.lock().unwrap();
                        g.current_tab = UITab::Image;
                        g.save_user_config();
                    }
                    if render_tab_btn(clay_scope, "tab_text", "Text", current_tab == UITab::Text, &arena, font_scale, &theme) {
                        let mut g = state.lock().unwrap();
                        g.current_tab = UITab::Text;
                        g.save_user_config();
                    }
                });

                let mut spacer = Declaration::<Texture2D, ()>::new();
                spacer.layout().width(grow!()).end();
                clay_scope.with(&spacer, |_| {});

                let mut persist_group = Declaration::<Texture2D, ()>::new();
                persist_group
                    .layout()
                    .direction(LayoutDirection::LeftToRight)
                    .child_gap(10)
                    .padding(Padding::new(0, 0, 0, 4))
                    .end();
                clay_scope.with(&persist_group, |clay_scope| {
                    let save_id = clay_scope.id("btn_save_state");
                    let mut save_color = theme.cl_bg_section;
                    if clay_scope.pointer_over(save_id) {
                        save_color = theme.cl_primary_hover;
                        if mouse_pressed {
                            let mut g = state.lock().unwrap();
                            let label = match g.current_tab {
                                UITab::Text => format!("Text: {}", *g.text_content),
                                UITab::Image => g.custom_image_path.as_ref().map(|p| (**p).clone()).unwrap_or_else(|| "Image".to_string()),
                                _ => "State".to_string(),
                            };
                            let new_state = g.capture_state(&label);
                            let mut states = (*g.saved_states).clone();
                            states.push(new_state);
                            g.saved_states = Arc::new(states);
                            g.save_persistence();
                        }
                    }
                    let mut save_btn = Declaration::<Texture2D, ()>::new();
                    save_btn
                        .id(save_id)
                        .layout()
                        .padding(Padding::new(12, 12, 6, 6))
                        .end()
                        .background_color(save_color)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&save_btn, |clay| {
                        clay.text(
                            arena.push(format!("{}   SAVE", ICON_COPY)),
                            clay_layout::text::TextConfig::new()
                                .font_size((14.0 * font_scale) as u16)
                                .color(theme.cl_text_main)
                                .end(),
                        );
                    });

                    let load_id = clay_scope.id("btn_load_state");
                    let mut load_color = theme.cl_bg_section;
                    if clay_scope.pointer_over(load_id) {
                        load_color = theme.cl_primary_hover;
                        if mouse_pressed {
                            let mut g = state.lock().unwrap();
                            g.load_dialog_open = !g.load_dialog_open;
                        }
                    }
                    let mut load_btn = Declaration::<Texture2D, ()>::new();
                    load_btn
                        .id(load_id)
                        .layout()
                        .padding(Padding::new(12, 12, 6, 6))
                        .end()
                        .background_color(load_color)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&load_btn, |clay| {
                        clay.text(
                            arena.push(format!("{}   LOAD", ICON_LAYERS)),
                            clay_layout::text::TextConfig::new()
                                .font_size((14.0 * font_scale) as u16)
                                .color(theme.cl_text_main)
                                .end(),
                        );
                        });
                    });
                });
            });

            let mut content_area = Declaration::<Texture2D, ()>::new();
            content_area
                .layout()
                .width(grow!())
                .height(grow!())
                .direction(LayoutDirection::LeftToRight)
                .padding(Padding::new(standard_margin, standard_margin, standard_margin, 0))
                .child_gap(16)
                .end();
            clay_scope.with(&content_area, |clay_scope| {
                let current_tab = state.lock().unwrap().current_tab;

                // Column 1: Grid (Always on Left, Grows)
                let mut col1 = Declaration::<Texture2D, ()>::new();
                col1.layout()
                    .width(grow!())
                    .height(grow!())
                    .direction(LayoutDirection::TopToBottom)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .child_gap(12)
                    .end();

                clay_scope.with(&col1, |clay_scope| {
                    let mut canvas_box = Declaration::<Texture2D, ()>::new();
                    canvas_box
                        .id(clay_scope.id("canvas"))
                        .layout()
                        .width(grow!())
                        .height(grow!())
                        .end()
                        .background_color(theme.cl_bg_section)
                        .corner_radius()
                        .all(16.0 * font_scale)
                        .end();
                    clay_scope.with(&canvas_box, |_| {});

                    let mut label_box = Declaration::<Texture2D, ()>::new();
                    label_box
                        .layout()
                        .width(grow!())
                        .padding(Padding::all(10))
                        .direction(LayoutDirection::LeftToRight)
                        .child_gap(24)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                        .end();
                    clay_scope.with(&label_box, |clay_scope| {
                        let (vx, vy, mx, my, mstate) = {
                            let g = state.lock().unwrap();
                            (g.v_pos.x, g.v_pos.y, g.machine_pos.x, g.machine_pos.y, g.machine_state)
                        };
                        clay_scope.text(
                            arena.push(format!("V:   X: {:.1}   Y: {:.1}", vx, vy)),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(COLOR_USB_ICON)
                                .end(),
                        );
                        clay_scope.text(
                            arena.push(format!("M:   X: {:.1}   Y: {:.1}", mx, my)),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(theme.cl_text_sub)
                                .end(),
                        );
                        let status_color = match mstate.as_str() {
                            "Idle" => theme.cl_success,
                            "Alarm" => theme.cl_danger,
                            "Hold" => COLOR_SLIDER_POWER, // Yellowish
                            _ => theme.cl_text_sub,
                        };
                        clay_scope.text(
                            arena.push(format!("Status: {}", mstate)),
                            clay_layout::text::TextConfig::new()
                                .font_size((14.0 * font_scale) as u16)
                                .color(status_color)
                                .end(),
                        );

                        let tidy_id = clay_scope.id("tidy_grid");
                        let mut tidy_color = theme.cl_text_label;
                        if clay_scope.pointer_over(tidy_id) {
                            tidy_color = theme.cl_text_main;
                            if mouse_pressed {
                                let mut guard = state.lock().unwrap();
                                guard.paths.clear();
                            }
                        }
                        let mut tidy_btn = Declaration::<Texture2D, ()>::new();
                        tidy_btn
                            .id(tidy_id)
                            .layout()
                            .padding(Padding::all(6))
                            .direction(LayoutDirection::LeftToRight)
                            .child_gap(8)
                            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                            .end()
                            .background_color(theme.cl_bg_dark)
                            .corner_radius()
                            .all(6.0 * font_scale)
                            .end();
                        clay_scope.with(&tidy_btn, |clay| {
                            clay.text(
                                arena.push(format!("{}   TIDY", ICON_SWEEP)),
                                clay_layout::text::TextConfig::new()
                                    .font_size((12.0 * font_scale) as u16)
                                    .color(tidy_color)
                                    .end(),
                            );
                        });
                    });
                });

                // Column 2: Controls (SCROLLABLE)
                let col2_outer_id = clay_scope.id("controls_outer");
                let mut col2_outer = Declaration::<Texture2D, ()>::new();
                let col2_width = 400.0;
                col2_outer
                    .id(col2_outer_id)
                    .layout()
                    .width(fixed!(col2_width * font_scale))
                    .height(grow!())
                    .direction(LayoutDirection::LeftToRight)
                    .child_gap(0)
                    .end();

                clay_scope.with(&col2_outer, |clay_scope| {
                    let mut col2_scroll = Declaration::<Texture2D, ()>::new();
                    let col2_id = clay_scope.id("controls_column");
                    let c2_offset = state.lock().unwrap().col2_scroll_offset;

                    col2_scroll
                        .id(col2_id)
                        .layout()
                        .width(grow!())
                        .height(grow!())
                        .direction(LayoutDirection::TopToBottom)
                        .end()
                        .clip(
                            false,
                            true,
                            ClayVector2 {
                                x: 0.0,
                                y: c2_offset,
                            },
                        );

                    if clay_scope.pointer_over(col2_outer_id) || state.lock().unwrap().col2_dragging {
                        let mut skip_scroll = false;
                        {
                            let g = state.lock().unwrap();
                            if g.text_font_dropdown_open && clay_scope.pointer_over(clay_scope.id("font_dropdown_list")) {
                                skip_scroll = true;
                            }
                        }

                        if !skip_scroll {
                            let mut g = state.lock().unwrap();
                            g.col2_scroll_offset += scroll_delta.y * 40.0;
                            if g.col2_scroll_offset > 0.0 {
                                g.col2_scroll_offset = 0.0;
                            }
                        }
                    }

                    clay_scope.with(&col2_scroll, |clay_scope| match current_tab {
                    UITab::Manual => {
                        ui_manual::render_manual_left_subcol(
                            clay_scope,
                            &state,
                            &sections,
                            mouse_pressed,
                            &mut clipboard,
                            &arena,
                            font_scale,
                            &theme,
                        );
                        ui_manual::render_manual_right_col(
                            clay_scope,
                            &state,
                            &sections,
                            mouse_pos,
                            mouse_down,
                            mouse_pressed,
                            scroll_delta.y,
                            &mut clipboard,
                            &arena,
                            font_scale,
                            &theme,
                        );
                    }
                    UITab::Pattern => ui_test::render_test_controls(
                        clay_scope,
                        &state,
                        &sections,
                        mouse_pos,
                        mouse_down,
                        mouse_pressed,
                        scroll_delta.y,
                        &mut clipboard,
                        &arena,
                        font_scale,
                        &theme,
                    ),
                    UITab::Image => ui_image::render_image_controls(
                        clay_scope,
                        &state,
                        &sections,
                        mouse_pos,
                        mouse_down,
                        mouse_pressed,
                        scroll_delta.y,
                        &mut clipboard,
                        &arena,
                        font_scale,
                        &theme,
                    ),
                    UITab::Text => ui_text::render_text_controls(
                        clay_scope,
                        &state,
                        &sections,
                        mouse_pos,
                        mouse_down,
                        mouse_pressed,
                        scroll_delta.y,
                        &mut clipboard,
                        &arena,
                        font_scale,
                        &theme,
                    ),
                });

                let sb_area_id = clay_scope.id("col2_scrollbar_area");
                let mut sb_area = Declaration::<Texture2D, ()>::new();
                sb_area
                    .id(sb_area_id)
                    .layout()
                    .width(fixed!(8.0 * font_scale))
                    .height(grow!())
                    .padding(Padding::vertical(4))
                    .direction(LayoutDirection::TopToBottom)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
                    .end();

                let is_dragging = state.lock().unwrap().col2_dragging;
                if clay_scope.pointer_over(col2_outer_id) || is_dragging {
                    clay_scope.with(&sb_area, |clay_scope| {
                        let handle_height = 40.0 * font_scale;
                        let track_height = 800.0 * font_scale;
                        let max_scroll = 1500.0;
                        let scroll_ratio = (-c2_offset / max_scroll).clamp(0.0, 1.0);
                        let handle_y = ((track_height - handle_height) * scroll_ratio) as u16;

                        if clay_scope.pointer_over(sb_area_id) && mouse_pressed {
                            state.lock().unwrap().col2_dragging = true;
                        }

                        if state.lock().unwrap().col2_dragging {
                            if !mouse_down {
                                state.lock().unwrap().col2_dragging = false;
                            } else {
                                let dy = rl.get_mouse_delta().y;
                                let mut g = state.lock().unwrap();
                                // if we move handle down (dy > 0), content moves up (c2_offset decreases)
                                g.col2_scroll_offset -= dy * (max_scroll / track_height);
                                if g.col2_scroll_offset > 0.0 { g.col2_scroll_offset = 0.0; }
                                if g.col2_scroll_offset < -max_scroll { g.col2_scroll_offset = -max_scroll; }
                            }
                        }

                        let mut track = Declaration::<Texture2D, ()>::new();
                        track.layout()
                            .width(fixed!(2.0 * font_scale))
                            .height(grow!())
                            .padding(Padding::new(0, 0, handle_y, 0))
                            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
                            .end()
                            .background_color(theme.cl_text_sub);

                        clay_scope.with(&track, |clay_scope| {
                            let handle_id = clay_scope.id("col2_scroll_handle");
                            let mut handle = Declaration::<Texture2D, ()>::new();
                            handle.id(handle_id)
                                .layout()
                                .width(fixed!(6.0 * font_scale))
                                .height(fixed!(handle_height))
                                .end()
                                .background_color(theme.cl_text_main)
                                .corner_radius()
                                .all(3.0 * font_scale)
                                .end();
                            
                            clay_scope.with(&handle, |_| {});
                        });
                    });
                } else {
                    clay_scope.with(&sb_area, |_| {});
                }
            });
            });

            // FIXED BOTTOM AREA
            let mut bottom_area = Declaration::<Texture2D, ()>::new();
            bottom_area
                .layout()
                .width(grow!())
                .height(fixed!(bottom_bar_height))
                .direction(LayoutDirection::LeftToRight)
                .padding(Padding::all(0))
                .child_gap(0)
                .end();

            clay_scope.with(&bottom_area, |clay_scope| {
                let mstate = state.lock().unwrap().machine_state;
                let is_emergency = mstate == MachineState::Alarm || mstate == MachineState::Hold;
                let mut estop_b_color = if is_emergency {
                    theme.cl_success
                } else {
                    theme.cl_danger
                };
                let estop_b_id = clay_scope.id("estop_bottom");

                if clay_scope.pointer_over(estop_b_id) {
                    estop_b_color = if is_emergency {
                        COLOR_SUCCESS_LIGHT
                    } else {
                        COLOR_DANGER_HOVER
                    };
                    if mouse_pressed {
                        let mut g = state.lock().unwrap();
                        g.send_command(crate::gcode::CMD_FEED_HOLD.to_string());
                        g.send_command(crate::gcode::CMD_LASER_OFF.to_string());
                        g.send_command(crate::gcode::CMD_SOFT_RESET.to_string());
                    }
                }

                let is_collapsed = state.lock().unwrap().bottom_bar_height <= 65.0;
                let estop_size = if is_collapsed { 60.0 } else { 140.0 } * font_scale;
                let estop_font_size = if is_collapsed { 12.0 } else { 24.0 } * font_scale;

                let mut estop_b = Declaration::<Texture2D, ()>::new();
                estop_b
                    .id(estop_b_id)
                    .layout()
                    .width(fixed!(estop_size))
                    .height(fixed!(estop_size))
                    .direction(LayoutDirection::LeftToRight)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end()
                    .background_color(estop_b_color);
                clay_scope.with(&estop_b, |clay_scope| {
                    clay_scope.text(
                        "E-STOP",
                        clay_layout::text::TextConfig::new()
                            .font_size(estop_font_size as u16)
                            .color(theme.cl_text_main)
                            .end(),
                    );
                });

                crate::ui_components::render_log(
                    clay_scope,
                    &state,
                    scroll_delta.into(),
                    &arena,
                    font_scale,
                    &theme,
                    mouse_pressed,
                );
            });
        });
    });

            let load_dialog_open = state.lock().unwrap().load_dialog_open;
            if load_dialog_open {
                let mut overlay = Declaration::<Texture2D, ()>::new();
                overlay
                    .id(clay_scope.id("load_state_overlay"))
                    .layout()
                    .width(fixed!(render_width))
                    .height(fixed!(render_height))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end()
                    .floating()
                    .attach_to(clay_layout::elements::FloatingAttachToElement::Root)
                    .z_index(1001)
                    .end()
                    .background_color(theme.cl_overlay);

                clay_scope.with(&overlay, |clay_scope| {
                    let mut dialog_box = Declaration::<Texture2D, ()>::new();
                    dialog_box
                        .layout()
                        .width(fixed!(500.0 * font_scale))
                        .height(fixed!(400.0 * font_scale))
                        .padding(Padding::all(20))
                        .direction(LayoutDirection::TopToBottom)
                        .child_gap(16)
                        .end()
                        .background_color(theme.cl_bg_section)
                        .corner_radius()
                        .all(16.0 * font_scale)
                        .end();

                    clay_scope.with(&dialog_box, |clay_scope| {
                        let mut header = Declaration::<Texture2D, ()>::new();
                        header.layout().width(grow!()).direction(LayoutDirection::LeftToRight).end();
                        clay_scope.with(&header, |clay_scope| {
                            clay_scope.text(
                                "SAVED STATES",
                                clay_layout::text::TextConfig::new()
                                    .font_size((18.0 * font_scale) as u16)
                                    .color(theme.cl_primary)
                                    .end(),
                            );
                            let mut spacer = Declaration::<Texture2D, ()>::new();
                            spacer.layout().width(grow!()).end();
                            clay_scope.with(&spacer, |_| {});

                            let close_id = clay_scope.id("btn_close_load");
                            let mut close_color = theme.cl_text_sub;
                            if clay_scope.pointer_over(close_id) {
                                close_color = theme.cl_text_main;
                                if mouse_pressed {
                                    state.lock().unwrap().load_dialog_open = false;
                                }
                            }
                            let mut close_btn = Declaration::<Texture2D, ()>::new();
                            close_btn.id(close_id).layout().padding(Padding::all(4)).end();
                            clay_scope.with(&close_btn, |clay| {
                                clay.text(
                                    "CLOSE",
                                    clay_layout::text::TextConfig::new()
                                        .font_size((14.0 * font_scale) as u16)
                                        .color(close_color)
                                        .end(),
                                );
                            });
                        });

                        let saved_states = state.lock().unwrap().saved_states.clone();
                        let mut list_scroll = Declaration::<Texture2D, ()>::new();
                        let scroll_id = clay_scope.id("load_list_scroll");
                        list_scroll
                            .id(scroll_id)
                            .layout()
                            .width(grow!())
                            .height(grow!())
                            .direction(LayoutDirection::TopToBottom)
                            .child_gap(8)
                            .end()
                            .clip(false, true, ClayVector2::default());

                        clay_scope.with(&list_scroll, |clay_scope| {
                            if saved_states.is_empty() {
                                clay_scope.text(
                                    "No saved states found.",
                                    clay_layout::text::TextConfig::new()
                                        .font_size((14.0 * font_scale) as u16)
                                        .color(theme.cl_text_sub)
                                        .end(),
                                );
                            }
                            for (idx, s) in saved_states.iter().enumerate().rev() {
                                let item_id = clay_scope.id(arena.push(format!("load_item_{}", idx)));
                                let mut item_bg = theme.cl_bg_dark;
                                if clay_scope.pointer_over(item_id) {
                                    item_bg = theme.cl_primary_hover;
                                }

                                let mut item_row = Declaration::<Texture2D, ()>::new();
                                item_row
                                    .id(item_id)
                                    .layout()
                                    .width(grow!())
                                    .padding(Padding::all(10))
                                    .direction(LayoutDirection::LeftToRight)
                                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                                    .child_gap(12)
                                    .end()
                                    .background_color(item_bg)
                                    .corner_radius()
                                    .all(8.0 * font_scale)
                                    .end();

                                clay_scope.with(&item_row, |clay_scope| {
                                    let del_id = clay_scope.id(arena.push(format!("del_item_{}", idx)));
                                    let mut del_color = theme.cl_text_sub;
                                    if clay_scope.pointer_over(del_id) {
                                        del_color = theme.cl_danger;
                                        if mouse_pressed {
                                            let mut g = state.lock().unwrap();
                                            let mut states = (*g.saved_states).clone();
                                            states.remove(idx);
                                            g.saved_states = Arc::new(states);
                                            g.save_persistence();
                                        }
                                    }

                                    let mut info_col = Declaration::<Texture2D, ()>::new();
                                    info_col.layout().width(grow!()).direction(LayoutDirection::TopToBottom).end();
                                    if clay_scope.pointer_over(item_id) && !clay_scope.pointer_over(del_id) && mouse_pressed {
                                        let mut g = state.lock().unwrap();
                                        g.apply_state(s);
                                        g.load_dialog_open = false;
                                    }

                                    clay_scope.with(&info_col, |clay_scope| {
                                        clay_scope.text(
                                            arena.push(s.label.clone()),
                                            clay_layout::text::TextConfig::new()
                                                .font_size((14.0 * font_scale) as u16)
                                                .color(theme.cl_text_main)
                                                .end(),
                                        );
                                        clay_scope.text(
                                            arena.push(s.timestamp.clone()),
                                            clay_layout::text::TextConfig::new()
                                                .font_size((10.0 * font_scale) as u16)
                                                .color(theme.cl_text_sub)
                                                .end(),
                                        );
                                    });

                                    let mut del_btn = Declaration::<Texture2D, ()>::new();
                                    del_btn.id(del_id).layout().padding(Padding::all(4)).end();
                                    clay_scope.with(&del_btn, |clay| {
                                        clay.text(
                                            ICON_TRASH,
                                            clay_layout::text::TextConfig::new()
                                                .font_size((16.0 * font_scale) as u16)
                                                .color(del_color)
                                                .end(),
                                        );
                                    });
                                });
                            }
                        });
                    });
                });
            }

        crate::ui_components::render_toasts(&mut clay_scope, &state, &arena, font_scale, mouse_pressed, &theme);
        let render_commands = clay_scope.end();

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::color::Color::BLACK);

        for command in render_commands {
            match command.config {
                RenderCommandConfig::Rectangle(config) => {
                    let color = raylib::color::Color::new(
                        config.color.r as u8,
                        config.color.g as u8,
                        config.color.b as u8,
                        config.color.a as u8,
                    );

                    let estop_id = unsafe {
                        clay_layout::bindings::Clay_GetElementId(clay_layout::bindings::Clay_String::from("estop_bottom")).id
                    };

                    if command.id == estop_id {
                        d.draw_poly(
                            raylib::math::Vector2::new(
                                command.bounding_box.x + command.bounding_box.width / 2.0,
                                command.bounding_box.y + command.bounding_box.height / 2.0,
                            ),
                            8,
                            command.bounding_box.width / 2.0,
                            22.5,
                            color,
                        );
                    } else if config.corner_radii.top_left > 0.0 {
                        d.draw_rectangle_rounded(
                            raylib::math::Rectangle::new(
                                command.bounding_box.x,
                                command.bounding_box.y,
                                command.bounding_box.width,
                                command.bounding_box.height,
                            ),
                            config.corner_radii.top_left
                                / (command.bounding_box.width.min(command.bounding_box.height) / 2.0),
                            8,
                            color,
                        );
                    } else {
                        d.draw_rectangle(
                            command.bounding_box.x as i32,
                            command.bounding_box.y as i32,
                            command.bounding_box.width as i32,
                            command.bounding_box.height as i32,
                            color,
                        );
                    }

                    let canvas_id = unsafe {
                        clay_layout::bindings::Clay_GetElementId(clay_layout::bindings::Clay_String::from("canvas")).id
                    };
                    if command.id == canvas_id {
                        canvas_rect_actual = raylib::math::Rectangle::new(
                            command.bounding_box.x,
                            command.bounding_box.y,
                            command.bounding_box.width,
                            command.bounding_box.height,
                        );
                        
                        let margin = 20.0;
                        let side = (canvas_rect_actual.width - margin * 2.0).min(canvas_rect_actual.height - margin * 2.0);
                        let draw_area = raylib::math::Rectangle::new(
                            canvas_rect_actual.x + (canvas_rect_actual.width - side) / 2.0,
                            canvas_rect_actual.y + (canvas_rect_actual.height - side) / 2.0,
                            side,
                            side,
                        );

                        // 1. Draw grid lines
                        for i in 0..=20 {
                            let offset = (i as f32 / 20.0) * side;
                            let is_major = i % 5 == 0;
                            let color = if is_major {
                                raylib::color::Color::new(
                                    theme.cl_grid_major.r as u8,
                                    theme.cl_grid_major.g as u8,
                                    theme.cl_grid_major.b as u8,
                                    80,
                                )
                            } else {
                                raylib::color::Color::new(
                                    theme.cl_grid_minor.r as u8,
                                    theme.cl_grid_minor.g as u8,
                                    theme.cl_grid_minor.b as u8,
                                    30,
                                )
                            };
                            let thickness = if is_major { 2.0 } else { 1.0 };
                            d.draw_line_ex(
                                raylib::math::Vector2::new(draw_area.x + offset, draw_area.y),
                                raylib::math::Vector2::new(draw_area.x + offset, draw_area.y + draw_area.height),
                                thickness,
                                color,
                            );
                            d.draw_line_ex(
                                raylib::math::Vector2::new(draw_area.x, draw_area.y + offset),
                                raylib::math::Vector2::new(draw_area.x + draw_area.width, draw_area.y + offset),
                                thickness,
                                color,
                            );
                        }

                        // 2. Draw cached preview
                        d.draw_texture_pro(
                            &preview_texture,
                            raylib::math::Rectangle::new(0.0, 0.0, 2000.0, -2000.0),
                            raylib::math::Rectangle::new(draw_area.x, draw_area.y, side, side),
                            raylib::math::Vector2::new(0.0, 0.0),
                            0.0,
                            raylib::color::Color::WHITE,
                        );

                        let guard = state.lock().unwrap();
                        // 3. Draw bounds
                        if guard.bounds.enabled {
                            let bx = draw_area.x + (guard.bounds.x / 400.0) * side;
                            let by = draw_area.y + draw_area.height - (guard.bounds.y / 400.0) * side - (guard.bounds.h / 400.0) * side;
                            d.draw_rectangle_lines_ex(
                                raylib::math::Rectangle::new(
                                    bx,
                                    by,
                                    (guard.bounds.w / 400.0) * side,
                                    (guard.bounds.h / 400.0) * side,
                                ),
                                2.0,
                                raylib::color::Color::new(
                                    theme.cl_bounds.r as u8,
                                    theme.cl_bounds.g as u8,
                                    theme.cl_bounds.b as u8,
                                    150,
                                ),
                            );
                        }

                        // 4. Draw real-time paths
                        for p in &guard.paths {
                            let start = raylib::math::Vector2::new(
                                draw_area.x + (p.x1 / 400.0) * side,
                                draw_area.y + draw_area.height - (p.y1 / 400.0) * side,
                            );
                            let end = raylib::math::Vector2::new(
                                draw_area.x + (p.x2 / 400.0) * side,
                                draw_area.y + draw_area.height - (p.y2 / 400.0) * side,
                            );
                            d.draw_line_ex(
                                start,
                                end,
                                2.0,
                                raylib::color::Color::new(
                                    theme.cl_path.r as u8,
                                    theme.cl_path.g as u8,
                                    theme.cl_path.b as u8,
                                    (p.intensity * 255.0) as u8,
                                ),
                            );
                        }

                        // 5. Draw laser head
                        let head_pos = raylib::math::Vector2::new(
                            draw_area.x + (guard.machine_pos.x / 400.0) * side,
                            draw_area.y + draw_area.height - (guard.machine_pos.y / 400.0) * side,
                        );
                        d.draw_circle_v(
                            head_pos,
                            5.0 * font_scale,
                            raylib::color::Color::new(
                                theme.cl_head.r as u8,
                                theme.cl_head.g as u8,
                                theme.cl_head.b as u8,
                                100,
                            ),
                        );
                        d.draw_circle_v(
                            head_pos,
                            2.0 * font_scale,
                            raylib::color::Color::new(
                                theme.cl_danger.r as u8,
                                theme.cl_danger.g as u8,
                                theme.cl_danger.b as u8,
                                255,
                            ),
                        );
                    }
                }
                RenderCommandConfig::Image(config) => {
                    let color = raylib::color::Color::new(
                        config.background_color.r as u8,
                        config.background_color.g as u8,
                        config.background_color.b as u8,
                        config.background_color.a as u8,
                    );
                    d.draw_texture_pro(
                        config.data,
                        raylib::math::Rectangle::new(
                            0.0,
                            0.0,
                            config.data.width as f32,
                            config.data.height as f32,
                        ),
                        raylib::math::Rectangle::new(
                            command.bounding_box.x,
                            command.bounding_box.y,
                            command.bounding_box.width,
                            command.bounding_box.height,
                        ),
                        raylib::math::Vector2::new(0.0, 0.0),
                        0.0,
                        color,
                    );
                }
                RenderCommandConfig::Text(config) => {
                    let text_str = config.text;
                    let color = raylib::color::Color::new(
                        config.color.r as u8,
                        config.color.g as u8,
                        config.color.b as u8,
                        config.color.a as u8,
                    );
                    let font_size = command.bounding_box.height;
                    let text_size = font.measure_text_ex(text_str, font_size, 0.0);

                    if text_str.chars().count() == 1 {
                        let rotation = if text_str == ICON_SPINNER { frame_time_total * 360.0 } else { 0.0 };
                        let center = raylib::math::Vector2::new(
                            command.bounding_box.x + command.bounding_box.width / 2.0,
                            command.bounding_box.y + command.bounding_box.height / 2.0,
                        );
                        let mut origin = raylib::math::Vector2::new(text_size.x / 2.0, text_size.y / 2.0);
                        if let Some(c) = text_str.chars().next() {
                            if c as u32 > 127 {
                                let nudge_factor = if c == '\u{f06e}' || c == '\u{f070}' { 0.22 } else { 0.10 };
                                origin.x += font_size * nudge_factor;
                            }
                        }
                        d.draw_text_pro(&font, text_str, center, origin, rotation, font_size, 0.0, color);
                    } else {
                        let pos = raylib::math::Vector2::new(
                            command.bounding_box.x,
                            command.bounding_box.y,
                        );
                        d.draw_text_ex(&font, text_str, pos, font_size, 0.0, color);
                    }
                }
                RenderCommandConfig::ScissorStart() => unsafe {
                    raylib::ffi::BeginScissorMode(
                        command.bounding_box.x as i32,
                        command.bounding_box.y as i32,
                        command.bounding_box.width as i32,
                        command.bounding_box.height as i32,
                    );
                },
                RenderCommandConfig::ScissorEnd() => unsafe {
                    raylib::ffi::EndScissorMode();
                },
                RenderCommandConfig::Border(border) => {
                    let color = raylib::color::Color::new(
                        border.color.r as u8,
                        border.color.g as u8,
                        border.color.b as u8,
                        border.color.a as u8,
                    );
                    if border.width.top > 0 {
                        d.draw_rectangle(
                            command.bounding_box.x as i32,
                            command.bounding_box.y as i32,
                            command.bounding_box.width as i32,
                            border.width.top as i32,
                            color,
                        );
                    }
                    if border.width.bottom > 0 {
                        d.draw_rectangle(
                            command.bounding_box.x as i32,
                            (command.bounding_box.y + command.bounding_box.height) as i32 - border.width.bottom as i32,
                            command.bounding_box.width as i32,
                            border.width.bottom as i32,
                            color,
                        );
                    }
                    if border.width.left > 0 {
                        d.draw_rectangle(
                            command.bounding_box.x as i32,
                            command.bounding_box.y as i32,
                            border.width.left as i32,
                            command.bounding_box.height as i32,
                            color,
                        );
                    }
                    if border.width.right > 0 {
                        d.draw_rectangle(
                            (command.bounding_box.x + command.bounding_box.width) as i32 - border.width.right as i32,
                            command.bounding_box.y as i32,
                            border.width.right as i32,
                            command.bounding_box.height as i32,
                            color,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
