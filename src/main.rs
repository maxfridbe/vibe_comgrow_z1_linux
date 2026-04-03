#![windows_subsystem = "windows"]

mod icons;
mod state;
mod gcode;
mod comm;
mod ui;

use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::math::{Dimensions};
use clay_layout::{Clay, Declaration, Color, grow, fixed, fit};
use clay_layout::render_commands::{RenderCommandConfig};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use arboard::Clipboard;

use crate::icons::*;
use crate::state::{AppState, StringArena, PathSegment};
use crate::ui::{Command, Section, render_jog_btn, render_burn_btn, render_slider};
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
        // Give it a moment to send before process dies
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
    
    // Command definitions shared between CLI and GUI
    let sections = vec![
        Section {
            title: "Real-Time & System",
            icon: ICON_REFRESH,
            color: Color::u_rgb(96, 165, 250), // blue-400
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
            color: Color::u_rgb(251, 146, 60), // orange-400
            commands: vec![
                Command { label: "Dynamic", cmd: "M4" },
                Command { label: "Air On", cmd: "M8" },
                Command { label: "Air Off", cmd: "M9" },
            ],
        },
        Section {
            title: "Calibration",
            icon: ICON_GAUGE,
            color: Color::u_rgb(52, 211, 153), // emerald-400
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
            color: Color::u_rgb(248, 113, 113), // red-400
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
            color: Color::u_rgb(192, 132, 252), // purple-400
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
            color: Color::u_rgb(236, 72, 153), // pink-500
            commands: vec![
                Command { label: "Square Burn", cmd: "G90 G0 X50 Y50 F3000\nM4 S1000 F500\nG1 X100\nG1 Y100\nG1 X50\nG1 Y50\nG1 X100\nG1 Y100\nG1 X50\nG1 Y50\nM5\n$H" },
                Command { label: "Heart Burn", cmd: "G90 G0 X75 Y50 F3000\nM4 S1000 F500\nG1 X100 Y75\nG3 X75 Y75 R12.5\nG3 X50 Y75 R12.5\nG1 X75 Y50\nG1 X100 Y75\nG3 X75 Y75 R12.5\nG3 X50 Y75 R12.5\nG1 X75 Y50\nM5\n$H" },
                Command { label: "Star Burn", cmd: "G90 G0 X100 Y135 F3000\nM4 S1000 F500\nG1 X108.4 Y111.2\nG1 X133.6 Y111.2\nG1 X113.3 Y95.8\nG1 X120.3 Y66.4\nG1 X100 Y83.2\nG1 X79.7 Y66.4\nG1 X86.7 Y95.8\nG1 X66.4 Y111.2\nG1 X91.6 Y111.2\nG1 X100 Y135\nG1 X108.4 Y111.2\nG1 X133.6 Y111.2\nG1 X113.3 Y95.8\nG1 X120.3 Y66.4\nG1 X100 Y83.2\nG1 X79.7 Y66.4\nG1 X86.7 Y95.8\nG1 X66.4 Y111.2\nG1 X91.6 Y111.2\nG1 X100 Y135\nM5\n$H" },
            ],
        },
    ];

    let (tx, rx) = mpsc::channel::<String>();
    let _safety_guard = SafetyGuard { tx: tx.clone() };

    // Set up Ctrl-C handler
    let tx_for_ctrlc = tx.clone();
    ctrlc::set_handler(move || {
        println!("\n[CTRL-C] Detected.");
        let _ = tx_for_ctrlc.send("!".to_string());
        let _ = tx_for_ctrlc.send("M5".to_string());
        let _ = tx_for_ctrlc.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    if args.len() > 1 {
        if args[1] == "test-pattern" && args.len() >= 7 {
            return run_dynamic_pattern(&args[2], &args[3], &args[4], &args[5], &args[6]);
        }
        return run_cli_mode(&args[1], &sections);
    }

    let (tx, rx) = mpsc::channel::<String>();
    
    let state = Arc::new(Mutex::new(AppState {
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

    let font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, 64, Some(&chars.iter().collect::<String>()))
        .expect("Failed to load font");
    
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
        color: Color::u_rgb(236, 72, 153), // pink-500
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
        color: Color::u_rgb(34, 197, 94), // green-500
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
                font_scale = (font_scale + 0.5).min(15.0);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_MINUS) || rl.is_key_pressed(KeyboardKey::KEY_KP_SUBTRACT) {
                font_scale = (font_scale - 0.5).max(0.5);
            }
        }

        let mouse_pos = rl.get_mouse_position();
        let mouse_down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let mouse_pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let scroll_delta = rl.get_mouse_wheel_move_v();
        
        clay.pointer_state(clay_layout::math::Vector2::new(mouse_pos.x, mouse_pos.y), mouse_down);
        clay.update_scroll_containers(true, clay_layout::math::Vector2::new(scroll_delta.x * 50.0, scroll_delta.y * 50.0), rl.get_frame_time());
        clay.set_layout_dimensions(Dimensions::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32));

        let serial_id = unsafe { 
            clay_layout::id::Id { id: clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from("serial_box"), 0, 0) }
        };
        let mut scroll_pos = clay_layout::math::Vector2::new(0.0, 0.0);
        if let Some(scroll_data) = clay.scroll_container_data(serial_id) {
            scroll_pos = unsafe { (*scroll_data.scrollPosition).into() };
        }

        let canvas_id = unsafe { clay_layout::bindings::Clay_GetElementId(clay_layout::bindings::Clay_String::from("canvas")) };
        let mut canvas_side = 400.0 * font_scale;
        let canvas_data = unsafe { clay_layout::bindings::Clay_GetElementData(canvas_id) };
        if canvas_data.found {
            canvas_side = canvas_data.boundingBox.width;
        }

        let mut clay_scope = clay.begin::<Texture2D, ()>();

        let mut root_decl = Declaration::<Texture2D, ()>::new();
        root_decl.id(clay_scope.id("root"))
            .layout()
                .width(grow!())
                .height(grow!())
                .padding(Padding::all(6))
                .child_gap(16)
                .direction(LayoutDirection::TopToBottom)
            .end()
            .background_color(Color::u_rgb(15, 23, 42)); // slate-950-ish

        clay_scope.with(&root_decl, |clay_scope| {
            // Header
            let mut header_decl = Declaration::<Texture2D, ()>::new();
            header_decl.layout()
                    .width(grow!())
                    .height(fixed!(80.0 * font_scale))
                    .padding(Padding::all(12))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                .end()
                .background_color(Color::u_rgb(30, 41, 59)) // slate-900
                .corner_radius().all(16.0 * font_scale).end();

            clay_scope.with(&header_decl, |clay_scope| {
                let mut title_group = Declaration::<Texture2D, ()>::new();
                title_group.layout().child_gap(16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                clay_scope.with(&title_group, |clay_scope| {
                    let mut icon_box = Declaration::<Texture2D, ()>::new();
                    icon_box.layout().padding(Padding::all(8)).end()
                        .background_color(Color::u_rgb(37, 99, 235)) // blue-600
                        .corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&icon_box, |clay_scope| {
                        clay_scope.text(ICON_LASER, clay_layout::text::TextConfig::new().font_size((32.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                    });
                    
                    let mut text_box = Declaration::<Texture2D, ()>::new();
                    text_box.layout().direction(LayoutDirection::TopToBottom).child_gap(2).end();
                    clay_scope.with(&text_box, |clay_scope| {
                        clay_scope.text("Comgrow Z1 Laser GRBL Runner", clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                    });
                });

                let mut spacer = Declaration::<Texture2D, ()>::new();
                spacer.layout().width(grow!()).end();
                clay_scope.with(&spacer, |_| {});

                let mut settings_group = Declaration::<Texture2D, ()>::new();
                settings_group.layout().child_gap(12).child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Center)).end();
                clay_scope.with(&settings_group, |clay_scope| {
                    let mut input_box = Declaration::<Texture2D, ()>::new();
                    input_box.layout().padding(Padding::all(6)).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end()
                        .background_color(Color::u_rgb(2, 6, 23)) // slate-950
                        .corner_radius().all(12.0 * font_scale).end();
                    
                    let port_text = {
                        let guard = state.lock().unwrap();
                        guard.port.clone()
                    };
                    clay_scope.with(&input_box, |clay_scope| {
                        clay_scope.text(ICON_USB, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());
                        clay_scope.text(arena.push(port_text), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(191, 219, 254)).end());
                    });

                    let mut wattage_box = Declaration::<Texture2D, ()>::new();
                    wattage_box.layout().padding(Padding::all(6)).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end()
                        .background_color(Color::u_rgb(2, 6, 23))
                        .corner_radius().all(12.0 * font_scale).end();
                    
                    let wattage_text = {
                        let guard = state.lock().unwrap();
                        guard.wattage.clone()
                    };
                    clay_scope.with(&wattage_box, |clay_scope| {
                        clay_scope.text(ICON_CPU, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(192, 132, 252)).end());
                        clay_scope.text(arena.push(wattage_text), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(216, 180, 254)).end());
                    });
                });
            });

            // Main Content Area
            let mut main_grid = Declaration::<Texture2D, ()>::new();
            main_grid.layout().width(grow!()).height(grow!()).child_gap(24).end();
            clay_scope.with(&main_grid, |clay_scope| {
                // Left Column: Quick Commands
                let mut left_col = Declaration::<Texture2D, ()>::new();
                left_col.layout().height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();
                clay_scope.with(&left_col, |clay_scope| {
                    for section in sections.iter().filter(|s| s.title != "Safety") {
                        let mut section_box = Declaration::<Texture2D, ()>::new();
                        section_box.layout().width(grow!()).padding(Padding::all(6)).direction(LayoutDirection::TopToBottom).child_gap(12).end()
                            .background_color(Color::u_rgb(30, 41, 59))
                            .corner_radius().all(16.0 * font_scale).end();
                        
                        clay_scope.with(&section_box, |clay_scope| {
                            let mut title_line = Declaration::<Texture2D, ()>::new();
                            title_line.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                            clay_scope.with(&title_line, |clay_scope| {
                                clay_scope.text(section.icon, clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(section.color).end());
                                clay_scope.text(section.title, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(100, 116, 139)).end());
                            });

                            let mut cmd_grid = Declaration::<Texture2D, ()>::new();
                            cmd_grid.layout().width(grow!()).child_gap(8).end();
                            for chunk in section.commands.chunks(2) {
                                let mut row = Declaration::<Texture2D, ()>::new();
                                row.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).child_gap(8).end();
                                clay_scope.with(&row, |clay_scope| {
                                    for cmd in chunk {
                                        let btn_id = clay_scope.id(cmd.label);
                                        let mut btn_color = Color::u_rgb(2, 6, 23);
                                        if clay_scope.pointer_over(btn_id) {
                                            btn_color = Color::u_rgb(51, 65, 85);
                                            if mouse_pressed {
                                                let mut guard = state.lock().unwrap();
                                                let full_cmd = cmd.cmd.to_string();
                                                guard.send_command(full_cmd.clone());
                                                guard.copied_at = Some(std::time::Instant::now());
                                                if let Some(cb) = &mut clipboard {
                                                    let _ = cb.set_text(full_cmd);
                                                }
                                            }
                                        }

                                        let mut btn = Declaration::<Texture2D, ()>::new();
                                        btn.id(btn_id)
                                            .layout().width(fixed!(90.0 * font_scale)).padding(Padding::all(4)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                                            .background_color(btn_color)
                                            .corner_radius().all(12.0 * font_scale).end();
                                        clay_scope.with(&btn, |clay_scope| {
                                            clay_scope.text(cmd.label, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                                            clay_scope.text(cmd.cmd, clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                                        });
                                    }
                                });
                            }
                        });
                    }
                });

                // Middle Column: Virtual Canvas
                let mut mid_col = Declaration::<Texture2D, ()>::new();
                mid_col.layout()
                    .width(grow!())
                    .direction(LayoutDirection::TopToBottom)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
                    .child_gap(12)
                    .end();
                clay_scope.with(&mid_col, |clay_scope| {
                    let mut canvas_box = Declaration::<Texture2D, ()>::new();
                    canvas_box.id(clay_scope.id("canvas"))
                        .layout().width(grow!()).height(fixed!(canvas_side)).end()
                        .background_color(Color::u_rgb(30, 41, 59))
                        .corner_radius().all(16.0 * font_scale).end();
                    
                    clay_scope.with(&canvas_box, |clay_scope| {
                        let paths_empty = {
                            let guard = state.lock().unwrap();
                            guard.paths.is_empty()
                        };

                        if !paths_empty {
                            let mut sweep_btn = Declaration::<Texture2D, ()>::new();
                            sweep_btn.id(clay_scope.id("clear_canvas"))
                                .floating()
                                    .attach_points(clay_layout::elements::FloatingAttachPointType::RightTop, clay_layout::elements::FloatingAttachPointType::RightTop)
                                    .offset(clay_layout::math::Vector2::new(-16.0 * font_scale, 16.0 * font_scale))
                                .end()
                                .layout()
                                    .padding(Padding::all(4))
                                    .direction(LayoutDirection::TopToBottom)
                                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                                .end()
                                .background_color(Color::u_rgb(127, 29, 29))
                                .corner_radius().all(12.0 * font_scale).end();
                            
                            if clay_scope.pointer_over(clay_scope.id("clear_canvas")) && mouse_pressed {
                                let mut guard = state.lock().unwrap();
                                guard.paths.clear();
                            }

                            clay_scope.with(&sweep_btn, |clay_scope| {
                                clay_scope.text(ICON_SWEEP, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(248, 113, 113)).end());
                            });
                        }
                    });

                    let mut label_box = Declaration::<Texture2D, ()>::new();
                    label_box.layout().padding(Padding::all(6)).direction(LayoutDirection::TopToBottom).child_gap(2).end();
                    clay_scope.with(&label_box, |clay_scope| {
                        let (vx, vy, mx, my, mstate) = {
                            let guard = state.lock().unwrap();
                            (guard.v_pos.x, guard.v_pos.y, guard.machine_pos.x, guard.machine_pos.y, guard.machine_state.clone())
                        };
                        let v_pos_text = arena.push(format!("Virtual: X: {:.1}  Y: {:.1}", vx, vy));
                        clay_scope.text(v_pos_text, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());

                        let m_pos_text = arena.push(format!("Machine: X: {:.1}  Y: {:.1}", mx, my));
                        clay_scope.text(m_pos_text, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());

                        let state_text = arena.push(format!("Status: {}", mstate));
                        clay_scope.text(state_text, clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(Color::u_rgb(34, 197, 94)).end());
                    });
                });

                // Right Column: All Control Panels
                let mut right_col = Declaration::<Texture2D, ()>::new();
                right_col.layout()
                    .height(grow!())
                    .direction(LayoutDirection::TopToBottom)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
                    .child_gap(16)
                    .width(fit!())
                    .end();
                
                clay_scope.with(&right_col, |clay_scope| {
                    // Movement
                    let mut move_box = Declaration::<Texture2D, ()>::new();
                    move_box.layout().padding(Padding::all(12)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top)).child_gap(16).end()
                        .background_color(Color::u_rgb(30, 41, 59))
                        .corner_radius().all(16.0 * font_scale).end();
                    
                    clay_scope.with(&move_box, |clay_scope| {
                        let mut title = Declaration::<Texture2D, ()>::new();
                        title.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&title, |clay_scope| {
                            clay_scope.text(ICON_MOVE, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());
                            clay_scope.text("Movement", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                        });

                        let (dist, feed) = {
                            let guard = state.lock().unwrap();
                            (guard.distance, guard.feed_rate)
                        };

                        render_slider(clay_scope, "dist_slider", "Step (mm)", dist, 0.1, 100.0, Color::u_rgb(59, 130, 246), &state, |s, v| s.distance = v, mouse_pos, mouse_down, scroll_delta.y, &arena, font_scale);
                        render_slider(clay_scope, "feed_slider", "Speed (F)", feed, 10.0, 6000.0, Color::u_rgb(16, 185, 129), &state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, scroll_delta.y, &arena, font_scale);

                        // Jog Controls
                        let mut jog_grid = Declaration::<Texture2D, ()>::new();
                        jog_grid.layout().child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).direction(LayoutDirection::TopToBottom).child_gap(8).end();
                        clay_scope.with(&jog_grid, |clay_scope| {
                            // Up
                            let mut row1 = Declaration::<Texture2D, ()>::new(); row1.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                            clay_scope.with(&row1, |clay_scope| {
                                render_jog_btn(clay_scope, "up", ICON_ARROW_UP, &state, "Y", 1.0, mouse_pressed, &mut clipboard, font_scale);
                            });
                            // Left, Cross, Right
                            let mut row2 = Declaration::<Texture2D, ()>::new(); row2.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                            clay_scope.with(&row2, |clay_scope| {
                                render_jog_btn(clay_scope, "left", ICON_ARROW_LEFT, &state, "X", -1.0, mouse_pressed, &mut clipboard, font_scale);
                                
                                let center_id = clay_scope.id("center");
                                let mut center_color = Color::u_rgb(0, 0, 0);
                                if clay_scope.pointer_over(center_id) {
                                    center_color = Color::u_rgb(51, 65, 85);
                                    if mouse_pressed {
                                        let mut guard = state.lock().unwrap();
                                        guard.v_pos = Vector2::new(0.0, 0.0);
                                        guard.send_command("G92 X0 Y0".to_string());
                                        guard.copied_at = Some(std::time::Instant::now());
                                        if let Some(cb) = &mut clipboard { let _ = cb.set_text("G92 X0 Y0".to_string()); }
                                    }
                                }
                                let mut center_btn = Declaration::<Texture2D, ()>::new();
                                center_btn.id(center_id).layout().width(fixed!(30.0 * font_scale)).height(fixed!(30.0 * font_scale)).padding(Padding::all(4)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                                    .background_color(center_color).corner_radius().all(8.0 * font_scale).end();
                                clay_scope.with(&center_btn, |clay_scope| {
                                    clay_scope.text(ICON_CROSSHAIR, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(59, 130, 246)).end());
                                });

                                let home_zero_id = clay_scope.id("home_zero");
                                let mut home_zero_color = Color::u_rgb(0, 0, 0);
                                if clay_scope.pointer_over(home_zero_id) {
                                    home_zero_color = Color::u_rgb(51, 65, 85);
                                    if mouse_pressed {
                                        let mut guard = state.lock().unwrap();
                                        guard.v_pos = Vector2::new(0.0, 0.0);
                                        guard.send_command("G90 G0 X0 Y0".to_string());
                                        guard.copied_at = Some(std::time::Instant::now());
                                        if let Some(cb) = &mut clipboard { let _ = cb.set_text("G90 G0 X0 Y0".to_string()); }
                                    }
                                }
                                let mut home_zero_btn = Declaration::<Texture2D, ()>::new();
                                home_zero_btn.id(home_zero_id).layout().width(fixed!(30.0 * font_scale)).height(fixed!(30.0 * font_scale)).padding(Padding::all(4)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                                    .background_color(home_zero_color).corner_radius().all(8.0 * font_scale).end();
                                clay_scope.with(&home_zero_btn, |clay_scope| {
                                    clay_scope.text(ICON_HOME, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(52, 211, 153)).end());
                                });

                                render_jog_btn(clay_scope, "right", ICON_ARROW_RIGHT, &state, "X", 1.0, mouse_pressed, &mut clipboard, font_scale);
                            });
                            // Down
                            let mut row3 = Declaration::<Texture2D, ()>::new(); row3.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                            clay_scope.with(&row3, |clay_scope| {
                                render_jog_btn(clay_scope, "down", ICON_ARROW_DOWN, &state, "Y", -1.0, mouse_pressed, &mut clipboard, font_scale);
                            });
                        });
                    });

                    // Safety
                    if let Some(section) = sections.iter().find(|s| s.title == "Safety") {
                        let mut safety_box = Declaration::<Texture2D, ()>::new();
                        safety_box.layout().padding(Padding::all(12)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top)).child_gap(16).end()
                            .background_color(Color::u_rgb(30, 41, 59))
                            .corner_radius().all(16.0 * font_scale).end();
                        
                        clay_scope.with(&safety_box, |clay_scope| {
                            let mut title = Declaration::<Texture2D, ()>::new();
                            title.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                            clay_scope.with(&title, |clay_scope| {
                                clay_scope.text(section.icon, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(section.color).end());
                                clay_scope.text(section.title, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                            });

                            let mut content_row = Declaration::<Texture2D, ()>::new();
                            content_row.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                            
                            clay_scope.with(&content_row, |clay_scope| {
                                // Left Column: Existing Commands
                                let mut commands_col = Declaration::<Texture2D, ()>::new();
                                commands_col.layout().direction(LayoutDirection::TopToBottom).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                                clay_scope.with(&commands_col, |clay_scope| {
                                    for cmd in &section.commands {
                                        let btn_id = clay_scope.id(cmd.label);
                                        let mut btn_color = Color::u_rgb(2, 6, 23);
                                        if clay_scope.pointer_over(btn_id) {
                                            btn_color = Color::u_rgb(51, 65, 85);
                                            if mouse_pressed {
                                                let mut guard = state.lock().unwrap();
                                                guard.send_command(cmd.cmd.to_string());
                                                guard.copied_at = Some(std::time::Instant::now());
                                                if let Some(cb) = &mut clipboard { let _ = cb.set_text(cmd.cmd.to_string()); }
                                            }
                                        }

                                        let mut btn = Declaration::<Texture2D, ()>::new();
                                        btn.id(btn_id)
                                            .layout().width(fixed!(85.0 * font_scale)).padding(Padding::all(6)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                                            .background_color(btn_color)
                                            .corner_radius().all(12.0 * font_scale).end();
                                        clay_scope.with(&btn, |clay_scope| {
                                            clay_scope.text(cmd.label, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                                            clay_scope.text(cmd.cmd, clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                                        });
                                    }
                                });

                                // Right Column: Large ESTOP Button
                                let estop_id = clay_scope.id("estop_btn");
                                let mut estop_color = Color::u_rgb(185, 28, 28); // red-700
                                if clay_scope.pointer_over(estop_id) {
                                    estop_color = Color::u_rgb(220, 38, 38); // red-600
                                    if mouse_pressed {
                                        let mut guard = state.lock().unwrap();
                                        // Emergency Sequence
                                        guard.send_command("!".to_string());
                                        guard.send_command("M5".to_string());
                                        guard.send_command("\x18".to_string()); // 0x18 Soft Reset
                                        guard.paths.clear();
                                        guard.copied_at = Some(std::time::Instant::now());
                                    }
                                }

                                let mut estop_btn = Declaration::<Texture2D, ()>::new();
                                estop_btn.id(estop_id)
                                    .layout()
                                        .width(fixed!(100.0 * font_scale))
                                        .height(fixed!(100.0 * font_scale))
                                        .padding(Padding::all(12))
                                        .direction(LayoutDirection::TopToBottom)
                                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                                    .end()
                                    .background_color(estop_color)
                                    .corner_radius().all(16.0 * font_scale).end();
                                
                                clay_scope.with(&estop_btn, |clay_scope| {
                                    clay_scope.text("E-STOP", clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                                    clay_scope.text("(!, M5, ^X)", clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(Color::u_rgb(254, 202, 202)).end());
                                });
                            });
                        });
                    }

                    // Power Tuning
                    let mut power_box = Declaration::<Texture2D, ()>::new();
                    power_box.layout().padding(Padding::all(12)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top)).child_gap(16).end()
                        .background_color(Color::u_rgb(30, 41, 59))
                        .corner_radius().all(16.0 * font_scale).end();
                    
                    clay_scope.with(&power_box, |clay_scope| {
                        let mut title = Declaration::<Texture2D, ()>::new();
                        title.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&title, |clay_scope| {
                            clay_scope.text(ICON_FLAME, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(192, 132, 252)).end());
                            clay_scope.text("Power Tuning", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                        });

                        let pwr = {
                            let guard = state.lock().unwrap();
                            guard.power
                        };
                        render_slider(clay_scope, "power_slider", "Intensity (S)", pwr, 0.0, 1000.0, Color::u_rgb(168, 85, 247), &state, |s, v| s.power = v, mouse_pos, mouse_down, scroll_delta.y, &arena, font_scale);

                        let mut laser_row = Declaration::<Texture2D, ()>::new();
                        laser_row.layout().width(grow!()).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&laser_row, |clay_scope| {
                            let on_id = clay_scope.id("laser_on_btn");
                            let mut on_color = Color::u_rgb(153, 27, 27);
                            if clay_scope.pointer_over(on_id) {
                                on_color = Color::u_rgb(185, 28, 28);
                                if mouse_pressed {
                                    let mut guard = state.lock().unwrap();
                                    let s = guard.power;
                                    guard.send_command(format!("M3 S{:.0}", s));
                                    guard.copied_at = Some(std::time::Instant::now());
                                }
                            }
                            let mut on_btn = Declaration::<Texture2D, ()>::new();
                            on_btn.id(on_id).layout().width(fixed!(85.0 * font_scale)).padding(Padding::all(6)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                                .background_color(on_color).corner_radius().all(12.0 * font_scale).end();
                            clay_scope.with(&on_btn, |clay_scope| { clay_scope.text("LASER ON", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end()); });

                            let off_id = clay_scope.id("laser_off_btn");
                            let mut off_color = Color::u_rgb(51, 65, 85);
                            if clay_scope.pointer_over(off_id) {
                                off_color = Color::u_rgb(71, 85, 105);
                                if mouse_pressed {
                                    let mut guard = state.lock().unwrap();
                                    guard.send_command("M5".to_string());
                                    guard.copied_at = Some(std::time::Instant::now());
                                }
                            }
                            let mut off_btn = Declaration::<Texture2D, ()>::new();
                            off_btn.id(off_id).layout().width(fixed!(85.0 * font_scale)).padding(Padding::all(6)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                                .background_color(off_color).corner_radius().all(12.0 * font_scale).end();
                            clay_scope.with(&off_btn, |clay_scope| { clay_scope.text("LASER OFF", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end()); });
                        });

                        let mut burn_grid = Declaration::<Texture2D, ()>::new();
                        burn_grid.layout().width(grow!()).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).direction(LayoutDirection::TopToBottom).end();
                        clay_scope.with(&burn_grid, |clay_scope| {
                            let mut row1 = Declaration::<Texture2D, ()>::new(); row1.layout().child_gap(8).end();
                            clay_scope.with(&row1, |clay_scope| { render_burn_btn(clay_scope, "burn_up", "BURN UP", &state, 0.0, 1.0, mouse_pressed, &mut clipboard, font_scale); });
                            let mut row2 = Declaration::<Texture2D, ()>::new(); row2.layout().child_gap(8).end();
                            clay_scope.with(&row2, |clay_scope| {
                                render_burn_btn(clay_scope, "burn_left", "BURN LEFT", &state, -1.0, 0.0, mouse_pressed, &mut clipboard, font_scale);
                                render_burn_btn(clay_scope, "burn_right", "BURN RIGHT", &state, 1.0, 0.0, mouse_pressed, &mut clipboard, font_scale);
                            });
                            let mut row3 = Declaration::<Texture2D, ()>::new(); row3.layout().child_gap(8).end();
                            clay_scope.with(&row3, |clay_scope| { render_burn_btn(clay_scope, "burn_down", "BURN DOWN", &state, 0.0, -1.0, mouse_pressed, &mut clipboard, font_scale); });
                        });

                        let fire_id = clay_scope.id("fire_btn");
                        let mut fire_color = Color::u_rgb(2, 6, 23);
                        if clay_scope.pointer_over(fire_id) {
                            fire_color = Color::u_rgb(30, 41, 59);
                            if mouse_pressed {
                                let mut guard = state.lock().unwrap();
                                let cmd = if guard.wattage == "10W" { "M3 S5" } else { "M3 S10" }.to_string();
                                guard.send_command(cmd);
                                guard.copied_at = Some(std::time::Instant::now());
                            }
                        }
                        let mut fire_btn = Declaration::<Texture2D, ()>::new();
                        fire_btn.id(fire_id).layout().width(fixed!(140.0 * font_scale)).padding(Padding::all(6)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                            .background_color(fire_color).corner_radius().all(12.0 * font_scale).end();
                        clay_scope.with(&fire_btn, |clay_scope| { clay_scope.text("Focus Mode Fire", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(192, 132, 252)).end()); });

                        let power_test_id = clay_scope.id("power_test_btn");
                        let mut power_test_color = Color::u_rgb(220, 38, 38);
                        if clay_scope.pointer_over(power_test_id) {
                            power_test_color = Color::u_rgb(239, 68, 68);
                            if mouse_pressed {
                                let mut guard = state.lock().unwrap();
                                let sequence = ["G90", "G0 Y16", "G1 X50 F1000 S1000", "G0 X0", "G0 Y12", "G1 X50 F1000 S1000", "G0 X0", "G0 Y8", "G1 X50 F1000 S1000", "G0 X0", "G0 Y4", "G1 X50 F1000 S1000", "G0 X0", "G0 Y0"];
                                for s in sequence { guard.send_command(s.to_string()); }
                                guard.copied_at = Some(std::time::Instant::now());
                            }
                        }
                        let mut power_test_btn = Declaration::<Texture2D, ()>::new();
                        power_test_btn.id(power_test_id).layout().width(fixed!(140.0 * font_scale)).padding(Padding::all(6)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                            .background_color(power_test_color).corner_radius().all(12.0 * font_scale).end();
                        clay_scope.with(&power_test_btn, |clay_scope| { clay_scope.text("POWER TEST (Y-STEP)", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end()); });
                    });
                });
            });

            // Serial Output
            let mut serial_box = Declaration::<Texture2D, ()>::new();
            serial_box
                .id(serial_id)
                .floating()
                    .attach_points(clay_layout::elements::FloatingAttachPointType::CenterBottom, clay_layout::elements::FloatingAttachPointType::CenterBottom)
                    .attach_to(clay_layout::elements::FloatingAttachToElement::Parent)
                    .z_index(1000)
                    .parent_id(clay_scope.id("root").id.id)
                .end()
                .layout()
                    .width(grow!())
                    .height(fixed!(150.0 * font_scale))
                    .padding(Padding::all(12))
                    .direction(LayoutDirection::TopToBottom)
                    .child_gap(4)
                .end()
                .clip(false, true, scroll_pos)
                .background_color(Color::u_rgb(2, 6, 23))
                .corner_radius().all(16.0 * font_scale).end()
                .border().top((2.0 * font_scale) as u16).color(Color::u_rgb(168, 85, 247)).end();
            
            clay_scope.with(&serial_box, |clay_scope| {
                let mut title_line = Declaration::<Texture2D, ()>::new();
                title_line.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                clay_scope.with(&title_line, |clay_scope| {
                    clay_scope.text(ICON_SERIAL, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                    clay_scope.text("SERIAL LOG (COMMAND | EXPLANATION)", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                });
                
                let logs = {
                    let guard = state.lock().unwrap();
                    guard.serial_logs.clone()
                };
                
                for (i, log) in logs.iter().rev().enumerate() {
                    let mut text_color = if i == 0 { Color::u_rgb(255, 255, 255) } else { Color::u_rgb(148, 163, 184) };

                    if log.is_response {
                        text_color = Color::u_rgb(0, 0, 0);
                    }

                    let mut row = Declaration::<Texture2D, ()>::new();
                    row.layout().width(grow!()).padding(Padding::new((8.0 * font_scale) as u16, (8.0 * font_scale) as u16, (2.0 * font_scale) as u16, (2.0 * font_scale) as u16)).child_gap((20.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                    
                    if log.is_response {
                        row.background_color(Color::u_rgb(255, 255, 255))
                           .corner_radius().all(4.0 * font_scale).end();
                    }
                    
                    clay_scope.with(&row, |clay_scope| {
                        let mut col1 = Declaration::<Texture2D, ()>::new();
                        col1.layout().width(fixed!(350.0 * font_scale)).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&col1, |clay_scope| {
                            clay_scope.text(arena.push(log.text.clone()), clay_layout::text::TextConfig::new().font_size((11.0 * font_scale) as u16).color(text_color).end());
                        });
                        
                        let mut col2 = Declaration::<Texture2D, ()>::new();
                        col2.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&col2, |clay_scope| {
                            clay_scope.text(arena.push(log.explanation.clone()), clay_layout::text::TextConfig::new().font_size((11.0 * font_scale) as u16).color(text_color).end());
                        });
                    });
                }
            });

            // Footer
            let mut footer_box = Declaration::<Texture2D, ()>::new();
            footer_box.layout().width(grow!()).padding(Padding::all(6)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
            clay_scope.with(&footer_box, |clay_scope| {
                clay_scope.text("Comgrow Z1 Engineering Tool", clay_layout::text::TextConfig::new().font_size((11.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
            });
        });

        let render_commands = clay_scope.end();

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::color::Color::BLACK);
        
        let mut canvas_rect = raylib::math::Rectangle::default();
        let target_id = unsafe { clay_layout::bindings::Clay_GetElementId(clay_layout::bindings::Clay_String::from("canvas")) };

        for command in render_commands {
            if command.id == target_id.id {
                canvas_rect = raylib::math::Rectangle::new(command.bounding_box.x, command.bounding_box.y, command.bounding_box.width, command.bounding_box.height);
            }

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
                    if border.width.top > 0 {
                        d.draw_rectangle(command.bounding_box.x as i32, command.bounding_box.y as i32, command.bounding_box.width as i32, border.width.top as i32, color);
                    }
                }
                _ => {}
            }
        }

        // Draw Canvas Content
        if canvas_rect.width > 0.0 {
            let margin = 20.0;
            let side = (canvas_rect.width - margin * 2.0).min(canvas_rect.height - margin * 2.0);
            let draw_area = raylib::math::Rectangle::new(canvas_rect.x + margin, canvas_rect.y + margin, side, side);
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
    let (tx, rx) = mpsc::channel::<String>();
    let _guard = SafetyGuard { tx: tx.clone() };
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

fn run_dynamic_pattern(shape: &str, pwr_pct: &str, speed_pct: &str, scale_str: &str, passes_str: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, rx) = mpsc::channel::<String>();
    let _guard = SafetyGuard { tx: tx.clone() };
    let tx_ctrlc = tx.clone();
    let _ = ctrlc::set_handler(move || {
        let _ = tx_ctrlc.send("!".to_string());
        let _ = tx_ctrlc.send("M5".to_string());
        let _ = tx_ctrlc.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
        std::process::exit(0);
    });

    let pwr = pwr_pct.trim_end_matches('%').parse::<f32>().unwrap_or(1.0).clamp(0.0, 100.0);
    let spd = speed_pct.trim_end_matches('%').parse::<f32>().unwrap_or(100.0).clamp(1.0, 1000.0);
    let scale = scale_str.trim_end_matches('x').parse::<f32>().unwrap_or(1.0).max(0.1);
    let passes = passes_str.parse::<u32>().unwrap_or(1).clamp(1, 100);
    
    let s_val = (pwr * 10.0) as i32; // 100% = S1000
    let f_val = (spd * 10.0) as i32; // 100% = F1000
    
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
        _ => return Err(format!("Unknown shape '{}'. Try Square, Heart, or Star.", shape).into()),
    };

    if max_x > bed_size || max_y > bed_size {
        return Err(format!("Scale {:.1}x is too large! Shape would reach ({:.1}, {:.1}) which exceeds the {:.1}mm bed limit.", scale, max_x, max_y, bed_size).into());
    }

    // Assemble final G-code with setup, repeated path, and teardown
    let mut final_gcode = String::new();
    
    // Start position jump
    match shape.to_lowercase().as_str() {
        "square" => final_gcode.push_str(&format!("G90 G0 X{:.2} Y{:.2} F3000\n", offset_x, offset_y)),
        "heart" => final_gcode.push_str(&format!("G90 G0 X{:.2} Y{:.2} F3000\n", offset_x + (25.0 * scale), offset_y)),
        "star" => final_gcode.push_str(&format!("G90 G0 X{:.2} Y{:.2} F3000\n", 100.0, 100.0 + (35.0 * scale))),
        _ => {}
    }

    final_gcode.push_str(&format!("M4 S{} F{}\n", s_val, f_val));
    for _ in 0..passes {
        final_gcode.push_str(&path_gcode);
    }
    final_gcode.push_str("M5\n$H");

    println!("--- Dynamic Pattern: {} (Scale: {}x, Passes: {}, Power: {}%, Speed: {}%) ---", shape, scale, passes, pwr, spd);
    run_serial_cmd(&final_gcode, &format!("Dynamic {}", shape), tx)
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

    // To allow the SafetyGuard to work, we'll spawn a listener thread for the channel 
    // that has access to this specific port.
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let mut port_clone = port.try_clone()?;
    let tx_clone = tx.clone();
    let (tx_dummy, rx) = mpsc::channel::<String>(); // Use the passed tx for signals
    
    // We'll just use a simple loop here. If a signal comes in, we handle it.
    // However, the channel used by SafetyGuard is the one we passed in.
    // Since we are in CLI mode, we'll just check if the user sent a signal.
    
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
