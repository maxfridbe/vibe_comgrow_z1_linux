#![windows_subsystem = "windows"]

mod comm;
mod gcode;
mod icons;
mod state;
mod styles;
mod svg_helper;
mod ui;
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
use crate::state::{AppState, StringArena, UITab};
use crate::ui::{Command, Section, render_tab_btn};

const FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

trait FontMeasureEx {
    fn measure_text_ex(&self, text: &str, size: f32, spacing: f32) -> raylib::math::Vector2;
}

impl FontMeasureEx for raylib::prelude::Font {
    fn measure_text_ex(&self, text: &str, size: f32, spacing: f32) -> raylib::math::Vector2 {
        self.measure_text(text, size, spacing)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        is_absolute: true,
        port: "/dev/ttyUSB0".to_string(),
        wattage: "10W".to_string(),
        v_pos: Vector2::new(0.0, 0.0),
        machine_pos: Vector2::new(0.0, 0.0),
        machine_state: "Idle".to_string(),
        paths: Vec::new(),
        preview_paths: Vec::new(),
        preview_pattern: None,
        custom_svg_path: None,
        custom_image_path: None,
        last_command: String::new(),
        copied_at: None,
        serial_logs: std::collections::VecDeque::new(),
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
        text_content: "Comgrow Z1".to_string(),
        text_font: "Default".to_string(),
        text_is_bold: false,
        text_is_outline: false,
        text_letter_spacing: 0.0,
        text_line_spacing: 1.0,
        text_curve_steps: 10,
        text_lines_per_mm: 5.0,
        available_fonts: {
            let mut fonts = SystemSource::new().all_families().unwrap_or_default();
            fonts.sort();
            fonts
        },
        text_font_dropdown_open: false,
        text_font_scroll_offset: 0.0,
        is_text_input_active: false,
        current_preview_power: 1000.0,
        saved_states: Vec::new(),
        load_dialog_open: false,
    }));

    {
        let mut g = state.lock().unwrap();
        g.load_persistence();
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

    static mut MEASURE_FONT_PTR: *const Font = std::ptr::null();

    let mut clay = Clay::new(Dimensions::new(1280.0, 800.0));
    clay.set_measure_text_function(|text, config| {
        let size = config.font_size as f32;
        unsafe {
            if !MEASURE_FONT_PTR.is_null() {
                let f = &*MEASURE_FONT_PTR;
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
    let mut zoom_size: i32 = 64;
    let mut font = rl
        .load_font_from_memory(&thread, ".ttf", FONT_DATA, zoom_size, Some(&font_chars))
        .expect("Failed to load font");

    unsafe {
        MEASURE_FONT_PTR = &font as *const Font;
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
                    cmd: "$$",
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
                    cmd: "$30=1000",
                },
                Command {
                    label: "Laser Mode",
                    cmd: "$32=1",
                },
                Command {
                    label: "Y-Steps",
                    cmd: "$101=80",
                },
                Command {
                    label: "Rotary",
                    cmd: "$101=65",
                },
                Command {
                    label: "X-Steps",
                    cmd: "$100=80",
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
                    cmd: "$140=16",
                },
                Command {
                    label: "Hard Lmt",
                    cmd: "$21=1",
                },
                Command {
                    label: "Soft Lmt",
                    cmd: "$20=1",
                },
                Command {
                    label: "X-Travel",
                    cmd: "$130=400",
                },
                Command {
                    label: "Y-Travel",
                    cmd: "$131=400",
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
                zoom_size = (zoom_size + 16).min(128);
                font = rl
                    .load_font_from_memory(&thread, ".ttf", FONT_DATA, zoom_size, Some(&font_chars))
                    .expect("Failed to load font");
                unsafe {
                    MEASURE_FONT_PTR = &font as *const Font;
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_MINUS) {
                zoom_size = (zoom_size - 16).max(32);
                font = rl
                    .load_font_from_memory(&thread, ".ttf", FONT_DATA, zoom_size, Some(&font_chars))
                    .expect("Failed to load font");
                unsafe {
                    MEASURE_FONT_PTR = &font as *const Font;
                }
            }
        }

        let font_scale = zoom_size as f32 / 64.0;
        let _header_font_size = (zoom_size + 6) as u16;
        let _base_font_size = zoom_size as u16;

        let mouse_pos = rl.get_mouse_position();
        let frame_time_total = rl.get_time() as f32;

        let mouse_down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let mouse_pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let mut scroll_delta = rl.get_mouse_wheel_move_v();

        // Handle text input
        {
            let mut g = state.lock().unwrap();
            if g.is_text_input_active {
                while let Some(c) = rl.get_char_pressed() {
                    g.text_content.push(c);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                    g.text_content.pop();
                }
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
                let preview_thickness = (2000.0 / (400.0 * guard.text_lines_per_mm)).max(2.0);
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
                    let intensity = (p.intensity * 0.7 + 0.3).clamp(0.0, 1.0);
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

        let mut clay_scope = clay.begin::<Texture2D, ()>();

        // 1. MAIN APP LAYER
        let mut main_app_decl = Declaration::<Texture2D, ()>::new();
        main_app_decl
            .id(clay_scope.id("main_app_root"))
            .layout()
            .width(fixed!(render_width))
            .height(fixed!(render_height))
            .padding(Padding::all(6))
            .child_gap(12)
            .direction(LayoutDirection::TopToBottom)
            .end()
            .background_color(COLOR_BG_MAIN);

        clay_scope.with(&main_app_decl, |clay_scope| {
            let bottom_bar_height = 160.0 * font_scale;
            let standard_margin = (20.0 * font_scale) as u16;

            let mut header_decl = Declaration::<Texture2D, ()>::new();
            header_decl
                .layout()
                .width(grow!())
                .height(fixed!(40.0 * font_scale))
                .padding(Padding::all(6))
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                .end()
                .background_color(COLOR_BG_SECTION)
                .corner_radius()
                .all(8.0 * font_scale)
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
                        .background_color(COLOR_PRIMARY_HOVER)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&icon_box, |clay_scope| {
                        clay_scope.text(
                            ICON_LASER,
                            clay_layout::text::TextConfig::new()
                                .font_size((16.0 * font_scale) as u16)
                                .color(COLOR_TEXT_WHITE)
                                .end(),
                        );
                    });
                    clay_scope.text(
                        "COMGROW Z1",
                        clay_layout::text::TextConfig::new()
                            .font_size((16.0 * font_scale) as u16)
                            .color(COLOR_TEXT_WHITE)
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
                    let mut port_bg = COLOR_BG_DARK;
                    let mut port_text_color = COLOR_PORT_TEXT;
                    if clay_scope.pointer_over(port_h_id) {
                        port_bg = COLOR_PRIMARY_HOVER;
                        port_text_color = COLOR_TEXT_WHITE;
                        if mouse_pressed {
                            let mut g = state.lock().unwrap();
                            if g.port == "VIRTUAL" {
                                g.port = "/dev/ttyUSB0".to_string();
                            } else {
                                g.port = "VIRTUAL".to_string();
                                g.machine_state = "Idle".to_string();
                                g.machine_pos = Vector2::new(0.0, 0.0);
                            }
                        }
                    }
                    if port == "VIRTUAL" {
                        port_bg = COLOR_ACCENT_PURPLE_VIRTUAL;
                        port_text_color = COLOR_TEXT_WHITE;
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
                        .background_color(COLOR_BG_DARK)
                        .corner_radius()
                        .all(6.0 * font_scale)
                        .end();
                    clay_scope.with(&wattage_box, |clay_scope| {
                        clay_scope.text(
                            arena.push(format!("{}   {}", ICON_CPU, wattage)),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(COLOR_WATTAGE_TEXT)
                                .end(),
                        );
                    });

                    let estop_h_id = clay_scope.id("estop_header");
                    let mstate = { state.lock().unwrap().machine_state.clone() };
                    let is_emergency = mstate == "Alarm" || mstate == "Hold";
                    let mut estop_h_color = if is_emergency {
                        COLOR_SUCCESS
                    } else {
                        COLOR_DANGER_DARK
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
                            arena.push(format!("{}   E-STOP", ICON_FLAME)),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(COLOR_TEXT_WHITE)
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
                .padding(Padding::horizontal(standard_margin))
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Bottom))
                .end();

            clay_scope.with(&header_row, |clay_scope| {
                let mut tab_bar = Declaration::<Texture2D, ()>::new();
                tab_bar.layout().direction(LayoutDirection::LeftToRight).child_gap(10).end();
                clay_scope.with(&tab_bar, |clay_scope| {
                    let current_tab = state.lock().unwrap().current_tab.clone();
                    if render_tab_btn(clay_scope, "tab_manual", "Manual", current_tab == UITab::Manual, &arena, font_scale) {
                        state.lock().unwrap().current_tab = UITab::Manual;
                    }
                    if render_tab_btn(clay_scope, "tab_pattern", "Pattern", current_tab == UITab::Pattern, &arena, font_scale) {
                        state.lock().unwrap().current_tab = UITab::Pattern;
                    }
                    if render_tab_btn(clay_scope, "tab_image", "Image", current_tab == UITab::Image, &arena, font_scale) {
                        state.lock().unwrap().current_tab = UITab::Image;
                    }
                    if render_tab_btn(clay_scope, "tab_text", "Text", current_tab == UITab::Text, &arena, font_scale) {
                        state.lock().unwrap().current_tab = UITab::Text;
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
                    let mut save_color = COLOR_BG_SECTION;
                    if clay_scope.pointer_over(save_id) {
                        save_color = COLOR_PRIMARY_HOVER;
                        if mouse_pressed {
                            let mut g = state.lock().unwrap();
                            let label = match g.current_tab {
                                UITab::Text => format!("Text: {}", g.text_content),
                                UITab::Image => g.custom_image_path.clone().unwrap_or("Image".to_string()),
                                _ => "State".to_string(),
                            };
                            let new_state = g.capture_state(&label);
                            g.saved_states.push(new_state);
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
                                .color(COLOR_TEXT_WHITE)
                                .end(),
                        );
                    });

                    let load_id = clay_scope.id("btn_load_state");
                    let mut load_color = COLOR_BG_SECTION;
                    if clay_scope.pointer_over(load_id) {
                        load_color = COLOR_PRIMARY_HOVER;
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
                                .color(COLOR_TEXT_WHITE)
                                .end(),
                        );
                    });
                });
            });

            let mut content_area = Declaration::<Texture2D, ()>::new();
            content_area
                .layout()
                .width(grow!())
                .height(grow!())
                .direction(LayoutDirection::LeftToRight)
                .padding(Padding::all(standard_margin))
                .child_gap(16)
                .end();
            clay_scope.with(&content_area, |clay_scope| {
                let current_tab = state.lock().unwrap().current_tab.clone();

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
                        .background_color(COLOR_BG_SECTION)
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
                            (g.v_pos.x, g.v_pos.y, g.machine_pos.x, g.machine_pos.y, g.machine_state.clone())
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
                                .color(COLOR_TEXT_MUTED)
                                .end(),
                        );
                        let status_color = match mstate.as_str() {
                            "Idle" => COLOR_SUCCESS,
                            "Alarm" => COLOR_DANGER,
                            "Hold" => COLOR_SLIDER_POWER, // Yellowish
                            _ => COLOR_TEXT_MUTED,
                        };
                        clay_scope.text(
                            arena.push(format!("Status: {}", mstate)),
                            clay_layout::text::TextConfig::new()
                                .font_size((14.0 * font_scale) as u16)
                                .color(status_color)
                                .end(),
                        );

                        let tidy_id = clay_scope.id("tidy_grid");
                        let mut tidy_color = COLOR_TEXT_LABEL;
                        if clay_scope.pointer_over(tidy_id) {
                            tidy_color = COLOR_TEXT_WHITE;
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
                            .background_color(COLOR_BG_DARK)
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
                let mut col2_scroll = Declaration::<Texture2D, ()>::new();
                let col2_id = clay_scope.id("controls_column");
                let col2_width = 400.0;
                let c2_offset = { state.lock().unwrap().col2_scroll_offset };

                col2_scroll
                    .id(col2_id)
                    .layout()
                    .width(fixed!(col2_width * font_scale))
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

                if clay_scope.pointer_over(col2_id) {
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
                    ),
                });
            });

            // FIXED BOTTOM AREA
            let mut bottom_area = Declaration::<Texture2D, ()>::new();
            bottom_area
                .layout()
                .width(grow!())
                .height(fixed!(bottom_bar_height))
                .direction(LayoutDirection::LeftToRight)
                .child_gap(16)
                .end();

            clay_scope.with(&bottom_area, |clay_scope| {
                let mstate = { state.lock().unwrap().machine_state.clone() };
                let is_emergency = mstate == "Alarm" || mstate == "Hold";
                let mut estop_b_color = if is_emergency {
                    COLOR_SUCCESS
                } else {
                    COLOR_DANGER
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

                let mut estop_b = Declaration::<Texture2D, ()>::new();
                let estop_size = 140.0 * font_scale;
                estop_b
                    .id(estop_b_id)
                    .layout()
                    .width(fixed!(estop_size))
                    .height(fixed!(estop_size))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end()
                    .background_color(estop_b_color)
                    .corner_radius()
                    .all(estop_size / 2.0)
                    .end();
                clay_scope.with(&estop_b, |clay_scope| {
                    clay_scope.text(
                        arena.push(format!("{}   E-STOP", ICON_FLAME)),
                        clay_layout::text::TextConfig::new()
                            .font_size((24.0 * font_scale) as u16)
                            .color(COLOR_TEXT_WHITE)
                            .end(),
                    );
                });

                let mut log_box = Declaration::<Texture2D, ()>::new();
                let serial_id_node = clay_scope.id("serial_box");
                log_box
                    .id(serial_id_node)
                    .layout()
                    .width(grow!())
                    .height(grow!())
                    .padding(Padding::all(12))
                    .direction(LayoutDirection::TopToBottom)
                    .child_gap(4)
                    .end()
                    .background_color(COLOR_BG_DARK)
                    .corner_radius()
                    .all(16.0 * font_scale)
                    .end()
                    .border()
                    .top((2.0 * font_scale) as u16)
                    .color(COLOR_ACCENT_PURPLE_LIGHT)
                    .end();

                clay_scope.with(&log_box, |clay_scope| {
                    let mut title_row = Declaration::<Texture2D, ()>::new();
                    title_row
                        .layout()
                        .width(grow!())
                        .child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Center))
                        .child_gap(16)
                        .end();
                    clay_scope.with(&title_row, |clay_scope| {
                        clay_scope.text(
                            "SERIAL LOG",
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(COLOR_TEXT_DISABLED)
                                .end(),
                        );
                    });

                    let logs = state.lock().unwrap().serial_logs.clone();
                    let offset = { state.lock().unwrap().log_scroll_offset };
                    let mut log_scroll = Declaration::<Texture2D, ()>::new();
                    let log_scroll_id = clay_scope.id("log_scroll");
                    log_scroll
                        .id(log_scroll_id)
                        .layout()
                        .width(grow!())
                        .height(grow!())
                        .direction(LayoutDirection::TopToBottom)
                        .child_gap(2)
                        .end()
                        .clip(
                            false,
                            true,
                            ClayVector2 {
                                x: 0.0,
                                y: offset,
                            },
                        );

                    if clay_scope.pointer_over(log_scroll_id) {
                        let mut g = state.lock().unwrap();
                        g.log_scroll_offset += scroll_delta.y * 40.0;
                        if g.log_scroll_offset > 0.0 {
                            g.log_scroll_offset = 0.0;
                        }
                        let max_scroll = -(logs.len() as f32 * 20.0);
                        if g.log_scroll_offset < max_scroll {
                            g.log_scroll_offset = max_scroll;
                        }
                    }

                    clay_scope.with(&log_scroll, |clay_scope| {
                        for (i, log) in logs.iter().rev().take(1000).enumerate() {
                            let text_color = if log.is_response {
                                COLOR_TEXT_BLACK
                            } else if i == 0 {
                                COLOR_TEXT_WHITE
                            } else {
                                COLOR_TEXT_MUTED
                            };
                            let mut row = Declaration::<Texture2D, ()>::new();
                            row.layout()
                                .width(grow!())
                                .padding(Padding::horizontal(8))
                                .padding(Padding::vertical(2))
                                .child_gap(10)
                                .end();
                            if log.is_response {
                                row.background_color(COLOR_TEXT_WHITE).corner_radius().all(4.0 * font_scale).end();
                            }
                            clay_scope.with(&row, |clay_scope| {
                                clay_scope.text(
                                    arena.push(format!("[{}] {} {}", log.timestamp, log.text, log.explanation)),
                                    clay_layout::text::TextConfig::new()
                                        .font_size((11.0 * font_scale) as u16)
                                        .color(text_color)
                                        .end(),
                                );
                            });
                        }
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
                    .background_color(clay_layout::Color::rgba(0.0, 0.0, 0.0, 180.0));

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
                        .background_color(COLOR_BG_SECTION)
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
                                    .color(COLOR_PRIMARY)
                                    .end(),
                            );
                            let mut spacer = Declaration::<Texture2D, ()>::new();
                            spacer.layout().width(grow!()).end();
                            clay_scope.with(&spacer, |_| {});

                            let close_id = clay_scope.id("btn_close_load");
                            let mut close_color = COLOR_TEXT_MUTED;
                            if clay_scope.pointer_over(close_id) {
                                close_color = COLOR_TEXT_WHITE;
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
                                        .color(COLOR_TEXT_DISABLED)
                                        .end(),
                                );
                            }
                            for (idx, s) in saved_states.iter().enumerate().rev() {
                                let item_id = clay_scope.id(arena.push(format!("load_item_{}", idx)));
                                let mut item_bg = COLOR_BG_DARK;
                                if clay_scope.pointer_over(item_id) {
                                    item_bg = COLOR_PRIMARY_HOVER;
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
                                    let mut del_color = COLOR_TEXT_MUTED;
                                    if clay_scope.pointer_over(del_id) {
                                        del_color = COLOR_DANGER;
                                        if mouse_pressed {
                                            let mut g = state.lock().unwrap();
                                            g.saved_states.remove(idx);
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
                                                .color(COLOR_TEXT_WHITE)
                                                .end(),
                                        );
                                        clay_scope.text(
                                            arena.push(s.timestamp.clone()),
                                            clay_layout::text::TextConfig::new()
                                                .font_size((10.0 * font_scale) as u16)
                                                .color(COLOR_TEXT_DISABLED)
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
                });

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
                    }
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

                    if text_str == ICON_SPINNER {
                        let rotation = frame_time_total * 360.0; // 1 rotation per second
                        let center = raylib::math::Vector2::new(
                            command.bounding_box.x + command.bounding_box.width / 2.0,
                            command.bounding_box.y + command.bounding_box.height / 2.0,
                        );
                        let origin = raylib::math::Vector2::new(text_size.x / 2.0, text_size.y / 2.0);
                        d.draw_text_pro(&font, text_str, center, origin, rotation, font_size, 0.0, color);
                    } else {
                        let pos = raylib::math::Vector2::new(
                            command.bounding_box.x + (command.bounding_box.width - text_size.x) / 2.0,
                            command.bounding_box.y + (command.bounding_box.height - text_size.y) / 2.0,
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

        if canvas_rect_actual.width > 0.0 {
            let margin = 20.0;
            let side = (canvas_rect_actual.width - margin * 2.0).min(canvas_rect_actual.height - margin * 2.0);
            let draw_area = raylib::math::Rectangle::new(
                canvas_rect_actual.x + (canvas_rect_actual.width - side) / 2.0,
                canvas_rect_actual.y + (canvas_rect_actual.height - side) / 2.0,
                side,
                side,
            );

            // Draw grid lines
            // Thin lines every 20 (400 / 20 = 20 segments)
            for i in 0..=20 {
                let offset = (i as f32 / 20.0) * side;
                let is_major = i % 5 == 0; // Every 100 (5 * 20 = 100)
                let color = if is_major {
                    raylib::color::Color::new(255, 255, 255, 80)
                } else {
                    raylib::color::Color::new(255, 255, 255, 30)
                };
                let thickness = if is_major {
                    2.0
                } else {
                    1.0
                };

                // Vertical
                d.draw_line_ex(
                    raylib::math::Vector2::new(draw_area.x + offset, draw_area.y),
                    raylib::math::Vector2::new(draw_area.x + offset, draw_area.y + draw_area.height),
                    thickness,
                    color,
                );
                // Horizontal
                d.draw_line_ex(
                    raylib::math::Vector2::new(draw_area.x, draw_area.y + offset),
                    raylib::math::Vector2::new(draw_area.x + draw_area.width, draw_area.y + offset),
                    thickness,
                    color,
                );
            }
            let guard = state.lock().unwrap();
            if guard.bounds.enabled {
                let bx = draw_area.x + (guard.bounds.x / 400.0) * side;
                let by = draw_area.y + draw_area.height
                    - (guard.bounds.y / 400.0) * side
                    - (guard.bounds.h / 400.0) * side;
                let bw = (guard.bounds.w / 400.0) * side;
                let bh = (guard.bounds.h / 400.0) * side;
                d.draw_rectangle_lines_ex(
                    raylib::math::Rectangle::new(bx, by, bw, bh),
                    2.0,
                    raylib::color::Color::new(52, 211, 153, 150),
                );
            }
            for p in &guard.paths {
                let start = raylib::math::Vector2::new(
                    draw_area.x + (p.x1 / 400.0) * side,
                    draw_area.y + draw_area.height - (p.y1 / 400.0) * side,
                );
                let end = raylib::math::Vector2::new(
                    draw_area.x + (p.x2 / 400.0) * side,
                    draw_area.y + draw_area.height - (p.y2 / 400.0) * side,
                );
                d.draw_line_ex(start, end, 2.0, raylib::color::Color::new(255, 71, 87, (p.intensity * 255.0) as u8));
            }
            // Use the cached preview texture for high performance
            d.draw_texture_pro(
                &preview_texture,
                raylib::math::Rectangle::new(0.0, 0.0, 2000.0, -2000.0), // Negative height to flip Y correctly
                raylib::math::Rectangle::new(draw_area.x, draw_area.y, side, side),
                raylib::math::Vector2::new(0.0, 0.0),
                0.0,
                raylib::color::Color::WHITE,
            );

            let head_pos = raylib::math::Vector2::new(
                draw_area.x + (guard.machine_pos.x / 400.0) * side,
                draw_area.y + draw_area.height - (guard.machine_pos.y / 400.0) * side,
            );
            d.draw_circle_v(head_pos, 5.0 * font_scale, raylib::color::Color::new(59, 130, 246, 100));
            d.draw_circle_v(head_pos, 2.0 * font_scale, raylib::color::Color::RED);
        }
    }
    Ok(())
}

fn clay_scope_id(id: &str) -> clay_layout::id::Id {
    unsafe {
        clay_layout::id::Id {
            id: clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from(id), 0, 0),
        }
    }
}
