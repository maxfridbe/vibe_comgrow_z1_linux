#![windows_subsystem = "windows"]

mod icons;
mod state;
mod gcode;
mod comm;
mod ui;
mod ui_manual;
mod ui_test;
mod ui_svg;
mod svg_helper;

use clay_layout::layout::{Padding, LayoutDirection, Alignment, LayoutAlignmentX, LayoutAlignmentY};
use clay_layout::math::{Dimensions, Vector2 as ClayVector2};
use clay_layout::{Clay, Declaration, Color, grow, fixed};
use clay_layout::render_commands::{RenderCommandConfig};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use arboard::Clipboard;
use std::ffi::OsString;

mod cli_and_helpers;
mod virtual_device;

use crate::icons::*;
use crate::state::{AppState, StringArena, UITab};
use crate::ui::{Command, Section, render_tab_btn};
use crate::comm::start_serial_thread;
use crate::cli_and_helpers::*;

const FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    
    let sections = vec![
        Section {
            title: "Real-Time & System",
            icon: ICON_REFRESH,
            color: Color::u_rgb(96, 165, 250),
            commands: vec![
                Command { label: "Status", cmd: "?" },
                Command { label: "Home", cmd: "$H" },
                Command { label: "Settings", cmd: "$$" },
                Command { label: "Hold", cmd: "!" },
                Command { label: "Resume", cmd: "~" },
                Command { label: "Unlock", cmd: "$X" },
                Command { label: "Reset", cmd: "0x18" },
            ],
        },
        Section {
            title: "Laser & Air",
            icon: ICON_FLAME,
            color: Color::u_rgb(251, 146, 60),
            commands: vec![
                Command { label: "Dynamic", cmd: "M4" },
                Command { label: "Air On", cmd: "M8" },
                Command { label: "Air Off", cmd: "M9" },
            ],
        },
        Section {
            title: "Calibration",
            icon: ICON_GAUGE,
            color: Color::u_rgb(52, 211, 153),
            commands: vec![
                Command { label: "Max S", cmd: "$30=1000" },
                Command { label: "Laser Mode", cmd: "$32=1" },
                Command { label: "Y-Steps", cmd: "$101=80" },
                Command { label: "Rotary", cmd: "$101=65" },
                Command { label: "X-Steps", cmd: "$100=80" },
            ],
        },
        Section {
            title: "Safety",
            icon: ICON_SHIELD,
            color: Color::u_rgb(248, 113, 113),
            commands: vec![
                Command { label: "Gyro", cmd: "$140=16" },
                Command { label: "Hard Lmt", cmd: "$21=1" },
                Command { label: "Soft Lmt", cmd: "$20=1" },
                Command { label: "X-Travel", cmd: "$130=400" },
                Command { label: "Y-Travel", cmd: "$131=400" },
            ],
        },
        Section {
            title: "Modals",
            icon: ICON_LAYERS,
            color: Color::u_rgb(192, 132, 252),
            commands: vec![
                Command { label: "Abs", cmd: "G90" },
                Command { label: "Inc", cmd: "G91" },
                Command { label: "mm", cmd: "G21" },
                Command { label: "inch", cmd: "G20" },
            ],
        },
        Section {
            title: "Test Patterns",
            icon: ICON_GAUGE,
            color: Color::u_rgb(236, 72, 153),
            commands: vec![
                Command { label: "Square", cmd: "" },
                Command { label: "Heart", cmd: "" },
                Command { label: "Star", cmd: "" },
                Command { label: "Car", cmd: "" },
                Command { label: "Stars8", cmd: "" },
                Command { label: "Stars9", cmd: "" },
            ],
        },
    ];

    if args.len() > 1 {
        if args[1] == "test-pattern" {
            let os_args: Vec<OsString> = std::env::args_os().collect();
            return run_dynamic_pattern_cli(&os_args[2..]);
        }
        return run_cli_mode(&args[1], &sections);
    }

    let (tx, rx) = mpsc::channel::<String>();
    let tx_for_ctrlc = tx.clone();
    ctrlc::set_handler(move || {
        let _ = tx_for_ctrlc.send("!".to_string());
        let _ = tx_for_ctrlc.send("M5".to_string());
        let _ = tx_for_ctrlc.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    let state = Arc::new(Mutex::new(AppState {
        current_tab: UITab::Manual,
        distance: 10.0,
        feed_rate: 1000.0,
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
        machine_state: "Unknown".to_string(),
        paths: Vec::new(),
        last_command: String::new(),
        copied_at: None,
        serial_logs: std::collections::VecDeque::new(),
        tx,
    }));

    start_serial_thread(Arc::clone(&state), rx);

    let (mut rl, thread) = raylib::init()
        .size(1280, 800)
        .title("Comgrow Z1 Laser GRBL Runner")
        .resizable()
        .build();

    rl.set_exit_key(None);
    rl.set_target_fps(60);

    let mut chars: Vec<char> = (32..127).map(|c| c as u8 as char).collect();
    let icons_list: &[&str] = &[
        ICON_TERMINAL, ICON_MOVE, ICON_POWER, ICON_HOME, ICON_UNLOCK, 
        ICON_REFRESH, ICON_SETTINGS, ICON_LAYERS, ICON_GAUGE, ICON_LASER,
        ICON_ARROW_UP, ICON_ARROW_DOWN, ICON_ARROW_LEFT, ICON_ARROW_RIGHT,
        ICON_CROSSHAIR, ICON_FLAME, ICON_USB, ICON_SHIELD, ICON_CPU, ICON_TRASH,
        ICON_COPY, ICON_SWEEP, ICON_SERIAL, ICON_CHECK,
    ];
    for &icon in icons_list {
        for c in icon.chars() {
            if !chars.contains(&c) { chars.push(c); }
        }
    }

    let font_chars: String = chars.iter().collect();
    let font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, 64, Some(&font_chars))
        .expect("Failed to load font");

    let mut clay = Clay::new(Dimensions::new(1280.0, 800.0));
    clay.set_measure_text_function(|text, config| {
        let size = config.font_size as f32;
        let width = text.len() as f32 * (size * 0.60);
        Dimensions::new(width, size)
    });
    let arena = StringArena::new();
    let mut clipboard = Clipboard::new().ok();
    let mut font_scale: f32 = 1.0;

    while !rl.window_should_close() {
        arena.clear();

        if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL) {
            if rl.is_key_pressed(KeyboardKey::KEY_EQUAL) || rl.is_key_pressed(KeyboardKey::KEY_KP_ADD) {
                font_scale = (font_scale + 0.1).min(5.0);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_MINUS) || rl.is_key_pressed(KeyboardKey::KEY_KP_SUBTRACT) {
                font_scale = (font_scale - 0.1).max(0.5);
            }
        }

        let mouse_pos = rl.get_mouse_position();
        let mouse_down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let mouse_pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let scroll_delta = rl.get_mouse_wheel_move_v();
        
        let screen_width = rl.get_screen_width() as f32;
        let screen_height = rl.get_screen_height() as f32;

        clay.set_layout_dimensions(Dimensions::new(screen_width, screen_height));
        clay.pointer_state(clay_layout::math::Vector2 { x: mouse_pos.x, y: mouse_pos.y }, mouse_down);
        clay.update_scroll_containers(true, clay_layout::math::Vector2 { x: scroll_delta.x * 50.0, y: scroll_delta.y * 50.0 }, rl.get_frame_time());

        let mut clay_scope = clay.begin::<Texture2D, ()>();

        let mut root_decl = Declaration::<Texture2D, ()>::new();
        root_decl.id(clay_scope.id("root"))
            .layout().width(grow!()).height(fixed!(screen_height - 12.0)).padding(Padding::all(6)).child_gap(12).direction(LayoutDirection::TopToBottom).end()
            .background_color(Color::u_rgb(15, 23, 42));

        clay_scope.with(&root_decl, |clay_scope| {
            let bottom_bar_height = 160.0 * font_scale;
            let standard_margin = (12.0 * font_scale) as u16;

            let mut header_decl = Declaration::<Texture2D, ()>::new();
            header_decl.layout().width(grow!()).height(fixed!(80.0 * font_scale)).padding(Padding::all(12)).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end()
                .background_color(Color::u_rgb(30, 41, 59))
                .corner_radius().all(16.0 * font_scale).end();

            clay_scope.with(&header_decl, |clay_scope| {
                let mut title_group = Declaration::<Texture2D, ()>::new();
                title_group.layout().child_gap(16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                clay_scope.with(&title_group, |clay_scope| {
                    let mut icon_box = Declaration::<Texture2D, ()>::new();
                    icon_box.layout().padding(Padding::all(8)).end().background_color(Color::u_rgb(37, 99, 235)).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&icon_box, |clay_scope| {
                        clay_scope.text(ICON_LASER, clay_layout::text::TextConfig::new().font_size((32.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                    });
                    clay_scope.text("Comgrow Z1 Laser GRBL Runner", clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                });

                let mut spacer = Declaration::<Texture2D, ()>::new(); spacer.layout().width(grow!()).end(); clay_scope.with(&spacer, |_| {});

                let mut settings_group = Declaration::<Texture2D, ()>::new();
                settings_group.layout().child_gap(12).child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Center)).end();
                clay_scope.with(&settings_group, |clay_scope| {
                    let (port, wattage) = { let g = state.lock().unwrap(); (g.port.clone(), g.wattage.clone()) };
                    let mut input_box = Declaration::<Texture2D, ()>::new();
                    input_box.layout().padding(Padding::all(6)).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end().background_color(Color::u_rgb(2, 6, 23)).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&input_box, |clay_scope| {
                        clay_scope.text(ICON_USB, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());
                        clay_scope.text(arena.push(port), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(191, 219, 254)).end());
                    });
                    let mut wattage_box = Declaration::<Texture2D, ()>::new();
                    wattage_box.layout().padding(Padding::all(6)).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end().background_color(Color::u_rgb(2, 6, 23)).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&wattage_box, |clay_scope| {
                        clay_scope.text(ICON_CPU, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(192, 132, 252)).end());
                        clay_scope.text(arena.push(wattage), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(216, 180, 254)).end());
                    });

                    let estop_h_id = clay_scope.id("estop_header");
                    let mut estop_h_color = Color::u_rgb(153, 27, 27); 
                    if clay_scope.pointer_over(estop_h_id) {
                        estop_h_color = Color::u_rgb(185, 28, 28);
                        if mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            guard.send_command("!".to_string());
                            guard.send_command("M5".to_string());
                            guard.send_command("0x18".to_string());
                            guard.paths.clear();
                        }
                    }
                    let mut estop_h_btn = Declaration::<Texture2D, ()>::new();
                    estop_h_btn.id(estop_h_id).layout().padding(Padding::new(12, 12, 6, 6)).end().background_color(estop_h_color).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&estop_h_btn, |clay_scope| {
                        clay_scope.text("E-STOP", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                    });
                });
            });

            let mut tab_bar = Declaration::<Texture2D, ()>::new();
            tab_bar.layout().width(grow!()).direction(LayoutDirection::LeftToRight).padding(Padding::horizontal(standard_margin)).child_gap(10).end();
            clay_scope.with(&tab_bar, |clay_scope| {
                let current_tab = state.lock().unwrap().current_tab.clone();
                if render_tab_btn(clay_scope, "tab_manual", "Manual", current_tab == UITab::Manual, font_scale) { state.lock().unwrap().current_tab = UITab::Manual; }
                if render_tab_btn(clay_scope, "tab_test", "Test", current_tab == UITab::Test, font_scale) { state.lock().unwrap().current_tab = UITab::Test; }
                if render_tab_btn(clay_scope, "tab_svg", "SVG", current_tab == UITab::SVG, font_scale) { state.lock().unwrap().current_tab = UITab::SVG; }
            });

            let mut content_area = Declaration::<Texture2D, ()>::new();
            content_area.layout().width(grow!()).height(grow!()).direction(LayoutDirection::LeftToRight).child_gap(12).end();
            clay_scope.with(&content_area, |clay_scope| {
                let current_tab = state.lock().unwrap().current_tab.clone();

                // Column 1: Grid (Always on Left, Grows)
                let mut col1 = Declaration::<Texture2D, ()>::new();
                col1.layout().width(grow!()).height(grow!()).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).child_gap(12).end();
                
                clay_scope.with(&col1, |clay_scope| {
                    let mut canvas_box = Declaration::<Texture2D, ()>::new();
                    canvas_box.id(clay_scope.id("canvas")).layout().width(grow!()).height(grow!()).end().background_color(Color::u_rgb(30, 41, 59)).corner_radius().all(16.0 * font_scale).end();
                    clay_scope.with(&canvas_box, |_| {});

                    let mut label_box = Declaration::<Texture2D, ()>::new();
                    label_box.layout().width(grow!()).padding(Padding::all(10)).direction(LayoutDirection::LeftToRight).child_gap(24).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                    clay_scope.with(&label_box, |clay_scope| {
                        let (vx, vy, mx, my, mstate) = { let g = state.lock().unwrap(); (g.v_pos.x, g.v_pos.y, g.machine_pos.x, g.machine_pos.y, g.machine_state.clone()) };
                        clay_scope.text(arena.push(format!("V: X:{:.1} Y:{:.1}", vx, vy)), clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());
                        clay_scope.text(arena.push(format!("M: X:{:.1} Y:{:.1}", mx, my)), clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                        clay_scope.text(arena.push(format!("Status: {}", mstate)), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(34, 197, 94)).end());
                    });
                });

                // Column 2: Controls (SCROLLABLE)
                let mut col2_scroll = Declaration::<Texture2D, ()>::new();
                let col2_id = clay_scope.id("controls_column");
                let col2_width = if current_tab == UITab::Manual { 750.0 } else { 350.0 };
                let c2_offset = { state.lock().unwrap().col2_scroll_offset };

                col2_scroll.id(col2_id).layout().width(fixed!(col2_width * font_scale)).height(grow!()).direction(LayoutDirection::TopToBottom).end()
                    .clip(false, true, ClayVector2 { x: 0.0, y: c2_offset });
                
                if clay_scope.pointer_over(col2_id) {
                    let mut g = state.lock().unwrap();
                    g.col2_scroll_offset += scroll_delta.y * 40.0;
                    if g.col2_scroll_offset > 0.0 { g.col2_scroll_offset = 0.0; }
                    // Bounds checking for scroll could be improved if we knew content height
                }

                clay_scope.with(&col2_scroll, |clay_scope| {
                    match current_tab {
                        UITab::Manual => {
                            let mut inner_row = Declaration::<Texture2D, ()>::new();
                            inner_row.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
                            clay_scope.with(&inner_row, |clay_scope| {
                                ui_manual::render_manual_left_col(clay_scope, &state, &sections, mouse_pressed, &mut clipboard, &arena, font_scale);
                                ui_manual::render_manual_right_col(clay_scope, &state, &sections, mouse_pos, mouse_down, mouse_pressed, scroll_delta.y, &mut clipboard, &arena, font_scale);
                            });
                        }
                        UITab::Test => ui_test::render_test_left_col(clay_scope, &state, &sections, mouse_pos, mouse_down, mouse_pressed, scroll_delta.y, &mut clipboard, &arena, font_scale),
                        UITab::SVG => ui_svg::render_svg_left_col(clay_scope, &state, mouse_pressed, &mut clipboard, &arena, font_scale),
                        _ => {}
                    }
                });
            });

            // FIXED BOTTOM AREA
            let mut bottom_area = Declaration::<Texture2D, ()>::new();
            bottom_area.layout().width(grow!()).height(fixed!(bottom_bar_height)).direction(LayoutDirection::LeftToRight).child_gap(16).end();

            clay_scope.with(&bottom_area, |clay_scope| {
                let mut log_box = Declaration::<Texture2D, ()>::new();
                log_box.layout().width(grow!()).height(grow!()).padding(Padding::all(12)).direction(LayoutDirection::TopToBottom).child_gap(4).end()
                    .background_color(Color::u_rgb(2, 6, 23)).corner_radius().all(16.0 * font_scale).end().border().top((2.0 * font_scale) as u16).color(Color::u_rgb(168, 85, 247)).end();
                
                clay_scope.with(&log_box, |clay_scope| {
                    let mut title_row = Declaration::<Texture2D, ()>::new();
                    title_row.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Center)).child_gap(16).end();
                    clay_scope.with(&title_row, |clay_scope| {
                        clay_scope.text("SERIAL LOG", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                        let port_id = clay_scope.id("port_toggle");
                        let current_port = { state.lock().unwrap().port.clone() };
                        let mut port_color = if current_port == "VIRTUAL" { Color::u_rgb(192, 132, 252) } else { Color::u_rgb(52, 211, 153) };
                        if clay_scope.pointer_over(port_id) {
                            port_color = Color::u_rgb(255, 255, 255);
                            if mouse_pressed {
                                let mut g = state.lock().unwrap();
                                if g.port == "VIRTUAL" { g.port = "/dev/ttyUSB0".to_string(); } 
                                else { g.port = "VIRTUAL".to_string(); g.machine_state = "Idle".to_string(); g.machine_pos = Vector2::new(0.0, 0.0); }
                            }
                        }
                        let mut port_btn = Declaration::<Texture2D, ()>::new();
                        port_btn.id(port_id).layout().padding(Padding::horizontal(8)).end();
                        clay_scope.with(&port_btn, |clay| {
                            clay.text(arena.push(format!("DEVICE: {}", current_port)), clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(port_color).end());
                        });
                    });

                    let logs = state.lock().unwrap().serial_logs.clone();
                    let offset = { state.lock().unwrap().log_scroll_offset };
                    let mut log_scroll = Declaration::<Texture2D, ()>::new();
                    let log_scroll_id = clay_scope.id("log_scroll");
                    log_scroll.id(log_scroll_id).layout().width(grow!()).height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(2).end().clip(false, true, ClayVector2 { x: 0.0, y: offset });
                    
                    if clay_scope.pointer_over(log_scroll_id) {
                        let mut g = state.lock().unwrap();
                        g.log_scroll_offset += scroll_delta.y * 40.0;
                        if g.log_scroll_offset > 0.0 { g.log_scroll_offset = 0.0; }
                        let max_scroll = -(logs.len() as f32 * 20.0);
                        if g.log_scroll_offset < max_scroll { g.log_scroll_offset = max_scroll; }
                    }

                    clay_scope.with(&log_scroll, |clay_scope| {
                        for (i, log) in logs.iter().rev().take(100).enumerate() {
                            let text_color = if log.is_response { Color::u_rgb(0, 0, 0) } else if i == 0 { Color::u_rgb(255, 255, 255) } else { Color::u_rgb(148, 163, 184) };
                            let mut row = Declaration::<Texture2D, ()>::new();
                            row.layout().width(grow!()).padding(Padding::horizontal(8)).padding(Padding::vertical(2)).child_gap(20).end();
                            if log.is_response { row.background_color(Color::u_rgb(255, 255, 255)).corner_radius().all(4.0 * font_scale).end(); }
                            clay_scope.with(&row, |clay_scope| {
                                clay_scope.text(arena.push(log.text.clone()), clay_layout::text::TextConfig::new().font_size((11.0 * font_scale) as u16).color(text_color).end());
                                clay_scope.text(arena.push(log.explanation.clone()), clay_layout::text::TextConfig::new().font_size((11.0 * font_scale) as u16).color(text_color).end());
                            });
                        }
                    });
                });

                let mut estop_b = Declaration::<Texture2D, ()>::new();
                let estop_b_id = clay_scope.id("estop_bottom");
                estop_b.id(estop_b_id).layout().width(fixed!(150.0 * font_scale)).height(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end().background_color(Color::u_rgb(220, 38, 38)).corner_radius().all(16.0 * font_scale).end();
                clay_scope.with(&estop_b, |clay_scope| {
                    clay_scope.text("E-STOP", clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                    if clay_scope.pointer_over(estop_b_id) && mouse_pressed {
                        let mut g = state.lock().unwrap(); g.send_command("!".to_string()); g.send_command("M5".to_string()); g.send_command("0x18".to_string());
                    }
                });
            });
        });

        let render_commands = clay_scope.end();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::color::Color::BLACK);
        
        let mut canvas_rect_actual = raylib::math::Rectangle::default();
        let target_id = unsafe { clay_layout::bindings::Clay_GetElementId(clay_layout::bindings::Clay_String::from("canvas")) };

        for command in render_commands {
            if command.id == target_id.id { canvas_rect_actual = raylib::math::Rectangle::new(command.bounding_box.x, command.bounding_box.y, command.bounding_box.width, command.bounding_box.height); }
            match command.config {
                RenderCommandConfig::Rectangle(rect) => {
                    let r = raylib::math::Rectangle::new(command.bounding_box.x, command.bounding_box.y, command.bounding_box.width, command.bounding_box.height);
                    let color = raylib::color::Color::new(rect.color.r as u8, rect.color.g as u8, rect.color.b as u8, rect.color.a as u8);
                    if rect.corner_radii.top_left > 0.0 { d.draw_rectangle_rounded(r, rect.corner_radii.top_left / (command.bounding_box.height / 2.0), 10, color); }
                    else { d.draw_rectangle(r.x as i32, r.y as i32, r.width as i32, r.height as i32, color); }
                }
                RenderCommandConfig::Text(text) => {
                    let sanitized = text.text.replace('\0', "");
                    let color = raylib::color::Color::new(text.color.r as u8, text.color.g as u8, text.color.b as u8, text.color.a as u8);
                    let text_size = font.measure_text(&sanitized, command.bounding_box.height, 0.0);
                    let pos = raylib::math::Vector2::new(command.bounding_box.x + (command.bounding_box.width - text_size.x) / 2.0, command.bounding_box.y + (command.bounding_box.height - text_size.y) / 2.0);
                    d.draw_text_ex(&font, &sanitized, pos, command.bounding_box.height, 0.0, color);
                }
                RenderCommandConfig::ScissorStart() => { unsafe { raylib::ffi::BeginScissorMode(command.bounding_box.x as i32, command.bounding_box.y as i32, command.bounding_box.width as i32, command.bounding_box.height as i32); } }
                RenderCommandConfig::ScissorEnd() => { unsafe { raylib::ffi::EndScissorMode(); } }
                RenderCommandConfig::Border(border) => {
                    let color = raylib::color::Color::new(border.color.r as u8, border.color.g as u8, border.color.b as u8, border.color.a as u8);
                    if border.width.top > 0 { d.draw_rectangle(command.bounding_box.x as i32, command.bounding_box.y as i32, command.bounding_box.width as i32, border.width.top as i32, color); }
                }
                _ => {}
            }
        }

        if canvas_rect_actual.width > 0.0 {
            let margin = 20.0;
            let side = (canvas_rect_actual.width - margin * 2.0).min(canvas_rect_actual.height - margin * 2.0);
            let draw_area = raylib::math::Rectangle::new(canvas_rect_actual.x + (canvas_rect_actual.width - side) / 2.0, canvas_rect_actual.y + (canvas_rect_actual.height - side) / 2.0, side, side);
            for i in 0..=10 {
                let x = draw_area.x + (i as f32 / 10.0) * side;
                let y = draw_area.y + (i as f32 / 10.0) * side;
                d.draw_line_v(raylib::math::Vector2::new(x, draw_area.y), raylib::math::Vector2::new(x, draw_area.y + draw_area.height), raylib::color::Color::new(255, 255, 255, 40));
                d.draw_line_v(raylib::math::Vector2::new(draw_area.x, y), raylib::math::Vector2::new(draw_area.x + draw_area.width, y), raylib::color::Color::new(255, 255, 255, 40));
            }
            let guard = state.lock().unwrap();
            for p in &guard.paths {
                let start = raylib::math::Vector2::new(draw_area.x + (p.x1 / 400.0) * side, draw_area.y + draw_area.height - (p.y1 / 400.0) * side);
                let end = raylib::math::Vector2::new(draw_area.x + (p.x2 / 400.0) * side, draw_area.y + draw_area.height - (p.y2 / 400.0) * side);
                d.draw_line_ex(start, end, 2.0, raylib::color::Color::new(255, 71, 87, (p.s / 1000.0 * 255.0) as u8));
            }
            let head_pos = raylib::math::Vector2::new(draw_area.x + (guard.machine_pos.x / 400.0) * side, draw_area.y + draw_area.height - (guard.machine_pos.y / 400.0) * side);
            d.draw_circle_v(head_pos, 5.0 * font_scale, raylib::color::Color::new(59, 130, 246, 100));
            d.draw_circle_v(head_pos, 2.0 * font_scale, raylib::color::Color::RED);
        }
    }
    Ok(())
}
