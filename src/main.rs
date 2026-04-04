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
use clay_layout::math::{Dimensions};
use clay_layout::{Clay, Declaration, Color, grow, fixed};
use clay_layout::render_commands::{RenderCommandConfig};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use arboard::Clipboard;
use std::ffi::OsString;

use crate::icons::*;
use crate::state::{AppState, StringArena, UITab};
use crate::ui::{Command, Section, render_tab_btn};
use crate::comm::start_serial_thread;

const FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

struct SafetyGuard {
    tx: mpsc::Sender<String>,
}

impl SafetyGuard {
    fn send_estop(&self) {
        println!("\n--- SAFETY: Sending Emergency Stop Sequence ---");
        let _ = self.tx.send("!".to_string());
        let _ = self.tx.send("M5".to_string());
        let _ = self.tx.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

impl Drop for SafetyGuard {
    fn drop(&mut self) {
        self.send_estop();
    }
}

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
                Command { label: "Square Burn", cmd: "G90 G0 X50 Y50 F3000\nM4 S1000 F500\nG1 X100\nG1 Y100\nG1 X50\nG1 Y50\nG1 X100\nG1 Y100\nG1 X50\nG1 Y50\nM5\n$H" },
                Command { label: "Heart Burn", cmd: "G90 G0 X75 Y50 F3000\nM4 S1000 F500\nG1 X100 Y75\nG3 X75 Y75 R12.5\nG3 X50 Y75 R12.5\nG1 X75 Y50\nG1 X100 Y75\nG3 X75 Y75 R12.5\nG3 X50 Y75 R12.5\nG1 X75 Y50\nM5\n$H" },
                Command { label: "Star Burn", cmd: "G90 G0 X100 Y135 F3000\nM4 S1000 F500\nG1 X108.4 Y111.2\nG1 X133.6 Y111.2\nG1 X113.3 Y95.8\nG1 X120.3 Y66.4\nG1 X100 Y83.2\nG1 X79.7 Y66.4\nG1 X86.7 Y95.8\nG1 X66.4 Y111.2\nG1 X91.6 Y111.2\nG1 X100 Y135\nG1 X108.4 Y111.2\nG1 X133.6 Y111.2\nG1 X113.3 Y95.8\nG1 X120.3 Y66.4\nG1 X100 Y83.2\nG1 X79.7 Y66.4\nG1 X86.7 Y95.8\nG1 X66.4 Y111.2\nG1 X91.6 Y111.2\nG1 X100 Y135\nM5\n$H" },
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
    let _safety_guard = SafetyGuard { tx: tx.clone() };

    let tx_for_ctrlc = tx.clone();
    ctrlc::set_handler(move || {
        println!("\n[CTRL-C] Detected.");
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
        ICON_SETTINGS, ICON_CHECK, ICON_ARROW_UP, ICON_ARROW_DOWN, 
        ICON_ARROW_LEFT, ICON_ARROW_RIGHT, ICON_CROSSHAIR, ICON_USB, 
        ICON_FLAME, ICON_GAUGE, ICON_SHIELD, ICON_REFRESH, ICON_CPU, 
        ICON_TRASH, ICON_LAYERS, ICON_COPY, ICON_LASER, ICON_SWEEP,
        ICON_SERIAL
    ];
    for icon in icons_list {
        chars.extend(icon.chars());
    }

    let font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, 64, Some(&chars.iter().collect::<String>())).expect("Failed to load font");
    let mut clay = Clay::new(Dimensions::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32));
    clay.set_measure_text_function(|text, config| {
        let size = config.font_size as f32;
        let width = text.len() as f32 * (size * 0.60);
        Dimensions::new(width, size)
    });
    let arena = StringArena::new();
    let mut clipboard = Clipboard::new().ok();
    let mut font_scale: f32 = 1.0;

    let mut sections = sections;
    sections.push(Section {
        title: "Y-JUMP (Absolute)",
        icon: ICON_LAYERS,
        color: Color::u_rgb(236, 72, 153),
        commands: vec![
            Command { label: "Y 16", cmd: "G90 G0 Y16" },
            Command { label: "Y 12", cmd: "G90 G0 Y12" },
            Command { label: "Y 8", cmd: "G90 G0 Y8" },
            Command { label: "Y 4", cmd: "G90 G0 Y4" },
        ],
    });
    sections.push(Section {
        title: "X-JUMP (Absolute)",
        icon: ICON_LAYERS,
        color: Color::u_rgb(34, 197, 94),
        commands: vec![
            Command { label: "X 100", cmd: "G90 G0 X100" },
            Command { label: "X 200", cmd: "G90 G0 X200" },
            Command { label: "X 300", cmd: "G90 G0 X300" },
            Command { label: "X 400", cmd: "G90 G0 X400" },
        ],
    });

    while !rl.window_should_close() {
        arena.clear();

        // Handle scaling
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

        // Use a consistent ID hashing for the canvas to ensure size detection works
        let canvas_id_val = clay_scope_id("canvas");
        let mut canvas_side = 400.0 * font_scale;
        let canvas_data = unsafe { clay_layout::bindings::Clay_GetElementData(canvas_id_val.id) };
        if canvas_data.found {
            canvas_side = canvas_data.boundingBox.width;
        }

        let mut clay_scope = clay.begin::<Texture2D, ()>();

        let mut root_decl = Declaration::<Texture2D, ()>::new();
        root_decl.id(clay_scope.id("root"))
            .layout().width(grow!()).height(fixed!(screen_height - 12.0)).padding(Padding::all(6)).child_gap(16).direction(LayoutDirection::TopToBottom).end()
            .background_color(Color::u_rgb(15, 23, 42));

        clay_scope.with(&root_decl, |clay_scope| {
            let bottom_bar_height = 160.0 * font_scale;
            let standard_margin = (20.0 * font_scale) as u16;

            // HEADER
            let mut header_decl = Declaration::<Texture2D, ()>::new();
            header_decl.layout().width(grow!()).height(fixed!(80.0 * font_scale)).padding(Padding::all(12)).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end()
                .background_color(Color::u_rgb(30, 41, 59))
                .corner_radius().all(16.0 * font_scale).end();

            clay_scope.with(&header_decl, |clay_scope| {
                let mut title_group = Declaration::<Texture2D, ()>::new();
                title_group.layout().child_gap(16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                clay_scope.with(&title_group, |clay_scope| {
                    let mut icon_box = Declaration::<Texture2D, ()>::new();
                    icon_box.layout().padding(Padding::all(8)).end()
                        .background_color(Color::u_rgb(37, 99, 235))
                        .corner_radius().all(12.0 * font_scale).end();
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

                    // Header E-STOP
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
                    estop_h_btn.id(estop_h_id).layout().padding(Padding::new(12, 12, 6, 6)).end()
                        .background_color(estop_h_color).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&estop_h_btn, |clay_scope| {
                        clay_scope.text("E-STOP", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                    });
                });
            });

            // TAB BAR
            let mut tab_bar = Declaration::<Texture2D, ()>::new();
            tab_bar.layout().width(grow!()).direction(LayoutDirection::LeftToRight).padding(Padding::horizontal(standard_margin)).child_gap(10).end();
            clay_scope.with(&tab_bar, |clay_scope| {
                let current_tab = state.lock().unwrap().current_tab.clone();
                if render_tab_btn(clay_scope, "tab_manual", "Manual", current_tab == UITab::Manual, font_scale) { state.lock().unwrap().current_tab = UITab::Manual; }
                if render_tab_btn(clay_scope, "tab_test", "Test", current_tab == UITab::Test, font_scale) { state.lock().unwrap().current_tab = UITab::Test; }
                if render_tab_btn(clay_scope, "tab_svg", "SVG", current_tab == UITab::SVG, font_scale) { state.lock().unwrap().current_tab = UITab::SVG; }
            });

            // CONTENT
            let mut content_area = Declaration::<Texture2D, ()>::new();
            content_area.layout().width(grow!()).height(grow!()).direction(LayoutDirection::LeftToRight).padding(Padding::new(standard_margin, standard_margin, standard_margin, (bottom_bar_height + 20.0) as u16)).child_gap(standard_margin).end();
            clay_scope.with(&content_area, |clay_scope| {
                let current_tab = state.lock().unwrap().current_tab.clone();

                // Col 1: Left
                match current_tab {
                    UITab::Manual => ui_manual::render_manual_left_col(clay_scope, &state, &sections, mouse_pressed, &mut clipboard, &arena, font_scale),
                    UITab::Test => ui_test::render_test_left_col(clay_scope, &state, &sections, mouse_pressed, &mut clipboard, &arena, font_scale),
                    UITab::SVG => ui_svg::render_svg_left_col(clay_scope, &state, mouse_pressed, &mut clipboard, &arena, font_scale),
                    _ => {}
                }

                // Col 2: Center
                let mut col2 = Declaration::<Texture2D, ()>::new();
                col2.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top)).child_gap(12).end();
                clay_scope.with(&col2, |clay_scope| {
                    let mut canvas_box = Declaration::<Texture2D, ()>::new();
                    canvas_box.id(clay_scope.id("canvas")).layout().width(grow!()).height(fixed!(canvas_side)).end().background_color(Color::u_rgb(30, 41, 59)).corner_radius().all(16.0 * font_scale).end();
                    clay_scope.with(&canvas_box, |_| {});

                    let mut label_box = Declaration::<Texture2D, ()>::new();
                    label_box.layout().padding(Padding::all(10)).direction(LayoutDirection::TopToBottom).child_gap(4).end();
                    clay_scope.with(&label_box, |clay_scope| {
                        let (vx, vy, mx, my, mstate) = { let g = state.lock().unwrap(); (g.v_pos.x, g.v_pos.y, g.machine_pos.x, g.machine_pos.y, g.machine_state.clone()) };
                        clay_scope.text(arena.push(format!("Virtual: X: {:.1}  Y: {:.1}", vx, vy)), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());
                        clay_scope.text(arena.push(format!("Machine: X: {:.1}  Y: {:.1}", mx, my)), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                        clay_scope.text(arena.push(format!("Status: {}", mstate)), clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(Color::u_rgb(34, 197, 94)).end());
                    });
                });

                // Col 3: Right
                match current_tab {
                    UITab::Manual => ui_manual::render_manual_right_col(clay_scope, &state, &sections, mouse_pos, mouse_down, mouse_pressed, scroll_delta.y, &mut clipboard, &arena, font_scale),
                    _ => {
                        let mut col3_spacer = Declaration::<Texture2D, ()>::new(); col3_spacer.layout().width(fixed!(400.0 * font_scale)).end(); clay_scope.with(&col3_spacer, |_| {});
                    }
                }
            });

            // DOCKED BOTTOM BAR
            let mut bottom_area = Declaration::<Texture2D, ()>::new();
            bottom_area
                .floating()
                    .attach_points(clay_layout::elements::FloatingAttachPointType::CenterBottom, clay_layout::elements::FloatingAttachPointType::CenterBottom)
                    .attach_to(clay_layout::elements::FloatingAttachToElement::Parent)
                .end()
                .layout()
                    .width(grow!())
                    .height(fixed!(bottom_bar_height))
                    .direction(LayoutDirection::LeftToRight)
                    .child_gap(16)
                    .padding(Padding::all(10))
                .end();

            clay_scope.with(&bottom_area, |clay_scope| {
                let mut log_box = Declaration::<Texture2D, ()>::new();
                let serial_id_node = clay_scope.id("serial_box");
                
                log_box.id(serial_id_node).layout().width(grow!()).height(grow!()).padding(Padding::all(12)).direction(LayoutDirection::TopToBottom).child_gap(4).end()
                    .background_color(Color::u_rgb(2, 6, 23)).corner_radius().all(16.0 * font_scale).end().border().top((2.0 * font_scale) as u16).color(Color::u_rgb(168, 85, 247)).end();
                
                clay_scope.with(&log_box, |clay_scope| {
                    clay_scope.text("SERIAL LOG", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                    let logs = state.lock().unwrap().serial_logs.clone();
                    for (i, log) in logs.iter().rev().enumerate() {
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
            let draw_area = raylib::math::Rectangle::new(canvas_rect_actual.x + margin, canvas_rect_actual.y + margin, side, side);
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
            let head_pos = raylib::math::Vector2::new(draw_area.x + (guard.v_pos.x / 400.0) * side, draw_area.y + draw_area.height - (guard.v_pos.y / 400.0) * side);
            d.draw_circle_v(head_pos, 5.0 * font_scale, raylib::color::Color::new(59, 130, 246, 100));
            d.draw_circle_v(head_pos, 2.0 * font_scale, raylib::color::Color::RED);
        }
    }
    Ok(())
}

fn run_cli_mode(target_label: &str, sections: &[Section]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, _rx) = mpsc::channel::<String>();
    let tx_ctrlc = tx.clone();
    let _ = ctrlc::set_handler(move || {
        let _ = tx_ctrlc.send("!".to_string());
        let _ = tx_ctrlc.send("M5".to_string());
        let _ = tx_ctrlc.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
        std::process::exit(0);
    });

    let cmd_str_owned: String;
    let cmd_str = if let Some(c) = sections.iter()
        .flat_map(|s| &s.commands)
        .find(|c| c.label.to_lowercase() == target_label.to_lowercase())
        .map(|c| c.cmd) 
    {
        c
    } else {
        cmd_str_owned = target_label.to_string();
        &cmd_str_owned
    };

    run_serial_cmd(cmd_str, target_label, tx)
}

fn run_dynamic_pattern_cli(args: &[OsString]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut pico_args = pico_args::Arguments::from_vec(args.to_vec());
    let shape: String = pico_args.free_from_str()?;
    let pwr_pct: String = pico_args.value_from_str("--power").unwrap_or_else(|_| "1%".to_string());
    let speed_pct: String = pico_args.value_from_str("--speed").unwrap_or_else(|_| "100%".to_string());
    let scale_str: String = pico_args.value_from_str("--scale").unwrap_or_else(|_| "1x".to_string());
    let passes_str: String = pico_args.value_from_str("--passes").unwrap_or_else(|_| "1".to_string());

    let (tx, _rx) = mpsc::channel::<String>();
    let tx_ctrlc = tx.clone();
    let _ = ctrlc::set_handler(move || {
        let _ = tx_ctrlc.send("!".to_string());
        let _ = tx_ctrlc.send("M5".to_string());
        let _ = tx_ctrlc.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
        std::process::exit(0);
    });

    let (cmd, label) = generate_pattern_gcode(&shape, &pwr_pct, &speed_pct, &scale_str, &passes_str)?;
    run_serial_cmd(&cmd, &label, tx)
}

fn generate_pattern_gcode(shape: &str, pwr_pct: &str, speed_pct: &str, scale_str: &str, passes_str: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr = pwr_pct.trim_end_matches('%').parse::<f32>().unwrap_or(1.0).clamp(0.0, 100.0);
    let spd = speed_pct.trim_end_matches('%').parse::<f32>().unwrap_or(100.0).clamp(1.0, 1000.0);
    let scale = scale_str.trim_end_matches('x').parse::<f32>().unwrap_or(1.0).max(0.1);
    let passes = passes_str.parse::<u32>().unwrap_or(1).clamp(1, 100);
    let s_val = (pwr * 10.0) as i32;
    let f_val = (spd * 10.0) as i32;
    let offset_x = 50.0;
    let offset_y = 50.0;
    let bed_size = 400.0;

    let (path_gcode, max_x, max_y) = match shape.to_lowercase().as_str() {
        "square" => {
            let size = 50.0 * scale;
            let x2 = offset_x + size;
            let y2 = offset_y + size;
            (format!("G1 X{:.2}\nG1 Y{:.2}\nG1 X{:.2}\nG1 Y{:.2}\n", x2, y2, offset_x, offset_y), x2, y2)
        },
        "heart" => {
            let w = 50.0 * scale;
            let h = 37.5 * scale;
            let r = 12.5 * scale;
            let start_x = offset_x + (w/2.0);
            let x_right = offset_x + w;
            let y_mid = offset_y + (h * 0.66);
            (format!("G1 X{:.2} Y{:.2}\nG3 X{:.2} Y{:.2} R{:.2}\nG3 X{:.2} Y{:.2} R{:.2}\nG1 X{:.2} Y{:.2}\n", 
                x_right, y_mid, start_x, y_mid, r, offset_x, y_mid, r, start_x, offset_y), x_right, y_mid + r)
        },
        "star" => {
            let cx = 100.0;
            let cy = 100.0;
            let pts = [
                (8.4, 11.2), (33.6, 11.2), (13.3, -4.2), (20.3, -33.6),
                (0.0, -16.8), (-20.3, -33.6), (-13.3, -4.2), (-33.6, 11.2), (-8.4, 11.2), (0.0, 35.0)
            ];
            let mut gcode = String::new();
            let mut mx = cx;
            let mut my = cy;
            for (dx, dy) in pts {
                let px = cx + (dx * scale);
                let py = cy + (dy * scale);
                gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", px, py));
                if px > mx { mx = px; }
                if py > my { my = py; }
            }
            (gcode, mx, my)
        },
        other => {
            let path = format!("assets/{}.svg", other);
            if std::path::Path::new(&path).exists() {
                svg_helper::load_svg_as_gcode(&path, scale, offset_x, offset_y)?
            } else {
                return Err(format!("Unknown shape '{}'. Try Square, Heart, Star, or a file in assets/.", shape).into());
            }
        }
    };

    if max_x > bed_size || max_y > bed_size {
        return Err(format!("Scale {:.1}x is too large! Shape would reach ({:.1}, {:.1}) which exceeds the {:.1}mm bed limit.", scale, max_x, max_y, bed_size).into());
    }

    let mut final_gcode = String::new();
    final_gcode.push_str("G90\n"); 
    final_gcode.push_str(&format!("M4 S{} F{}\n", s_val, f_val));
    for _ in 0..passes {
        final_gcode.push_str(&path_gcode);
    }
    final_gcode.push_str("M5\n$H");

    Ok((final_gcode, format!("Dynamic {} (Scale: {}x, Passes: {}, Power: {}%, Speed: {}%)", shape, scale, passes, pwr, spd)))
}

fn run_serial_cmd(cmd_str: &str, label: &str, tx: mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::io::{Write, Read};
    use crate::gcode::{decode_response, decode_gcode};

    fn get_ts() -> String {
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();
        let hh = (secs / 3600) % 24;
        let mm = (secs / 60) % 60;
        let ss = secs % 60;
        format!("{:02}:{:02}:{:02}", hh, mm, ss)
    }

    let port_name = "/dev/ttyUSB0";
    let baud_rate = 115200;

    println!("[{}] CLI: Mode = {}", get_ts(), label);

    let mut port = serialport::new(port_name, baud_rate)
        .timeout(std::time::Duration::from_millis(100))
        .open()?;

    println!("[{}] SERIAL: Connected to {}", get_ts(), port_name);

    for line in cmd_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        
        let full_cmd = if trimmed == "0x18" { "\x18".to_string() } else { format!("{}\n", trimmed) };
        let explanation = decode_gcode(trimmed);
        println!("[{}] SEND: {:?} | Interpreter: {}", get_ts(), trimmed, explanation);
        port.write_all(full_cmd.as_bytes())?;

        let mut serial_buf: Vec<u8> = vec![0; 1024];
        let mut accumulator = String::new();
        let start_time = std::time::Instant::now();
        let mut finished = false;

        while start_time.elapsed().as_secs() < 30 {
            if let Ok(t) = port.read(serial_buf.as_mut_slice()) {
                if t > 0 {
                    accumulator.push_str(&String::from_utf8_lossy(&serial_buf[..t]));
                    while let Some(pos) = accumulator.find('\n') {
                        let res_line = accumulator[..pos].trim().to_string();
                        accumulator.drain(..=pos);
                        if !res_line.is_empty() {
                            let explanation = decode_response(&res_line);
                            println!("[{}] RECV: {:?} | Interpreter: {}", get_ts(), res_line, explanation);
                            if res_line == "ok" || res_line.starts_with("error") {
                                finished = true;
                                break;
                            }
                        }
                    }
                }
            }
            if finished { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    Ok(())
}

fn clay_scope_id(id: &str) -> clay_layout::id::Id {
    unsafe { 
        clay_layout::id::Id { id: clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from(id), 0, 0) }
    }
}
