#![windows_subsystem = "windows"]

use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::math::{Vector2, Dimensions};
use clay_layout::{Clay, Declaration, Color, grow, fixed};
use clay_layout::render_commands::{RenderCommandConfig};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use arboard::Clipboard;

const FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

// Nerd Font Icon Constants
const ICON_TERMINAL: &str = "\u{f489}";
const ICON_MOVE: &str = "\u{f047}";
const ICON_POWER: &str = "\u{f0e7}";
const ICON_HOME: &str = "\u{f015}";
const ICON_UNLOCK: &str = "\u{f09c}";
const ICON_SETTINGS: &str = "\u{f013}";
const ICON_CHECK: &str = "\u{f058}";
const ICON_ARROW_UP: &str = "\u{f062}";
const ICON_ARROW_DOWN: &str = "\u{f063}";
const ICON_ARROW_LEFT: &str = "\u{f060}";
const ICON_ARROW_RIGHT: &str = "\u{f061}";
const ICON_CROSSHAIR: &str = "\u{f05b}";
const ICON_USB: &str = "\u{f287}";
const ICON_FLAME: &str = "\u{f06d}";
const ICON_GAUGE: &str = "\u{f0e4}";
const ICON_SHIELD: &str = "\u{f132}";
const ICON_REFRESH: &str = "\u{f021}";
const ICON_CPU: &str = "\u{f2db}";
const ICON_TRASH: &str = "\u{f1f8}";
const ICON_LAYERS: &str = "\u{f0c9}";
const ICON_COPY: &str = "\u{f0c5}";

struct PathSegment {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    s: f32,
}

struct AppState {
    distance: f32,
    feed_rate: f32,
    power: f32,
    port: String,
    wattage: String,
    v_pos: Vector2,
    paths: Vec<PathSegment>,
    last_command: String,
    copied_at: Option<std::time::Instant>,
    serial_logs: Vec<String>,
}

impl AppState {
    fn log_command(&mut self, cmd: String) {
        self.last_command = cmd.clone();
        self.serial_logs.push(cmd);
        if self.serial_logs.len() > 10 {
            self.serial_logs.remove(0);
        }
    }
}

struct StringArena {
    strings: RefCell<Vec<Box<str>>>,
}

impl StringArena {
    fn new() -> Self {
        Self { strings: RefCell::new(Vec::with_capacity(100)) }
    }

    fn push(&self, s: String) -> &str {
        let mut strings = self.strings.borrow_mut();
        let sanitized = s.replace('\0', "").into_boxed_str();
        let ptr = sanitized.as_ptr();
        let len = sanitized.len();
        strings.push(sanitized);
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }
    }

    fn clear(&self) {
        self.strings.borrow_mut().clear();
    }
}

struct Command {
    label: &'static str,
    cmd: &'static str,
}

struct Section {
    title: &'static str,
    icon: &'static str,
    color: Color,
    commands: Vec<Command>,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = Arc::new(Mutex::new(AppState {
        distance: 10.0,
        feed_rate: 1000.0,
        power: 100.0,
        port: "/dev/ttyUSB0".to_string(),
        wattage: "10W".to_string(),
        v_pos: Vector2::new(0.0, 0.0),
        paths: Vec::new(),
        last_command: String::new(),
        copied_at: None,
        serial_logs: Vec::new(),
    }));

    let (mut rl, thread) = raylib::init()
        .size(1280, 800)
        .title("Z1 Power Dash")
        .resizable()
        .build();

    rl.set_exit_key(None);
    rl.set_target_fps(60);

    let mut chars: Vec<char> = (32..127).map(|c| c as u8 as char).collect();
    let icons = [
        ICON_TERMINAL, ICON_MOVE, ICON_POWER, ICON_HOME, ICON_UNLOCK, 
        ICON_SETTINGS, ICON_CHECK, ICON_ARROW_UP, ICON_ARROW_DOWN, 
        ICON_ARROW_LEFT, ICON_ARROW_RIGHT, ICON_CROSSHAIR, ICON_USB, 
        ICON_FLAME, ICON_GAUGE, ICON_SHIELD, ICON_REFRESH, ICON_CPU, 
        ICON_TRASH, ICON_LAYERS, ICON_COPY
    ];
    for icon in icons {
        chars.extend(icon.chars());
    }

    let font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, 64, Some(&chars.iter().collect::<String>()))
        .expect("Failed to load font");
    
    let mut clay = Clay::new(Dimensions::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32));
    clay.set_measure_text_function(|text, config| {
        let size = config.font_size as f32;
        let width = text.len() as f32 * (size * 0.52);
        Dimensions::new(width, size)
    });
    let arena = StringArena::new();
    let mut clipboard = Clipboard::new().ok();
    let mut font_scale: f32 = 1.0;

    let sections = vec![
        Section {
            title: "Real-Time & System",
            icon: ICON_REFRESH,
            color: Color::u_rgb(96, 165, 250), // blue-400
            commands: vec![
                Command { label: "Status", cmd: "?" },
                Command { label: "Hold", cmd: "!" },
                Command { label: "Resume", cmd: "~" },
                Command { label: "Unlock", cmd: "$X" },
                Command { label: "Home", cmd: "$H" },
                Command { label: "Reset", cmd: "0x18" },
            ],
        },
        Section {
            title: "Laser & Air",
            icon: ICON_FLAME,
            color: Color::u_rgb(251, 146, 60), // orange-400
            commands: vec![
                Command { label: "Dynamic", cmd: "M4" },
                Command { label: "Constant", cmd: "M3" },
                Command { label: "Off", cmd: "M5" },
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
    ];

    while !rl.window_should_close() {
        arena.clear();

        // Handle scaling
        if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL) {
            if rl.is_key_pressed(KeyboardKey::KEY_EQUAL) || rl.is_key_pressed(KeyboardKey::KEY_KP_ADD) {
                font_scale = (font_scale + 0.1).min(3.0);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_MINUS) || rl.is_key_pressed(KeyboardKey::KEY_KP_SUBTRACT) {
                font_scale = (font_scale - 0.1).max(0.5);
            }
        }

        let mouse_pos = rl.get_mouse_position();
        let mouse_down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let mouse_pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        
        clay.pointer_state(Vector2::new(mouse_pos.x, mouse_pos.y), mouse_down);
        clay.set_layout_dimensions(Dimensions::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32));

        let mut clay_scope = clay.begin::<Texture2D, ()>();
        
        let mut root_decl = Declaration::<Texture2D, ()>::new();
        root_decl.id(clay_scope.id("root"))
            .layout()
                .width(grow!())
                .height(grow!())
                .padding(Padding::all((10.0 * font_scale) as u16))
                .child_gap((16.0 * font_scale) as u16)
                .direction(LayoutDirection::TopToBottom)
            .end()
            .background_color(Color::u_rgb(15, 23, 42)); // slate-950-ish

        clay_scope.with(&root_decl, |clay_scope| {
            // Header
            let mut header_decl = Declaration::<Texture2D, ()>::new();
            header_decl.layout()
                    .width(grow!())
                    .height(fixed!(80.0 * font_scale))
                    .padding(Padding::all((10.0 * font_scale) as u16))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end()
                .background_color(Color::u_rgb(30, 41, 59)) // slate-900
                .corner_radius().all(16.0 * font_scale).end();

            clay_scope.with(&header_decl, |clay_scope| {
                let mut title_group = Declaration::<Texture2D, ()>::new();
                title_group.layout().child_gap((16.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                clay_scope.with(&title_group, |clay_scope| {
                    let mut icon_box = Declaration::<Texture2D, ()>::new();
                    icon_box.layout().padding(Padding::all((8.0 * font_scale) as u16)).end()
                        .background_color(Color::u_rgb(37, 99, 235)) // blue-600
                        .corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&icon_box, |clay_scope| {
                        clay_scope.text(ICON_TERMINAL, clay_layout::text::TextConfig::new().font_size((32.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                    });
                    
                    let mut text_box = Declaration::<Texture2D, ()>::new();
                    text_box.layout().direction(LayoutDirection::TopToBottom).child_gap((2.0 * font_scale) as u16).end();
                    clay_scope.with(&text_box, |clay_scope| {
                        clay_scope.text("Z1 Power Dash", clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(241, 245, 249)).end());
                        clay_scope.text("Full Protocol Implementation", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(100, 116, 139)).end());
                    });
                });

                let mut settings_group = Declaration::<Texture2D, ()>::new();
                settings_group.layout().child_gap((12.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Center)).end();
                clay_scope.with(&settings_group, |clay_scope| {
                    let mut input_box = Declaration::<Texture2D, ()>::new();
                    input_box.layout().padding(Padding::new((8.0 * font_scale) as u16, (8.0 * font_scale) as u16, (6.0 * font_scale) as u16, (6.0 * font_scale) as u16)).child_gap((8.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end()
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
                    wattage_box.layout().padding(Padding::new((8.0 * font_scale) as u16, (8.0 * font_scale) as u16, (6.0 * font_scale) as u16, (6.0 * font_scale) as u16)).child_gap((8.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end()
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
            main_grid.layout().width(grow!()).height(grow!()).child_gap((24.0 * font_scale) as u16).end();
            clay_scope.with(&main_grid, |clay_scope| {
                // Left Column: Quick Commands
                let mut left_col = Declaration::<Texture2D, ()>::new();
                left_col.layout().width(fixed!(300.0 * font_scale)).height(grow!()).direction(LayoutDirection::TopToBottom).child_gap((16.0 * font_scale) as u16).end();
                clay_scope.with(&left_col, |clay_scope| {
                    for section in &sections {
                        let mut section_box = Declaration::<Texture2D, ()>::new();
                        section_box.layout().width(grow!()).padding(Padding::all((10.0 * font_scale) as u16)).direction(LayoutDirection::TopToBottom).child_gap((12.0 * font_scale) as u16).end()
                            .background_color(Color::u_rgb(30, 41, 59))
                            .corner_radius().all(16.0 * font_scale).end();
                        
                        clay_scope.with(&section_box, |clay_scope| {
                            let mut title_line = Declaration::<Texture2D, ()>::new();
                            title_line.layout().child_gap((8.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                            clay_scope.with(&title_line, |clay_scope| {
                                clay_scope.text(section.icon, clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(section.color).end());
                                clay_scope.text(section.title, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(100, 116, 139)).end());
                            });

                            let mut cmd_grid = Declaration::<Texture2D, ()>::new();
                            cmd_grid.layout().width(grow!()).child_gap((8.0 * font_scale) as u16).end();
                            // Clay doesn't have a built-in grid, we'll just wrap or use fixed columns
                            // For simplicity, let's do rows of 2
                            for chunk in section.commands.chunks(2) {
                                let mut row = Declaration::<Texture2D, ()>::new();
                                row.layout().width(grow!()).child_gap((8.0 * font_scale) as u16).end();
                                clay_scope.with(&row, |clay_scope| {
                                    for cmd in chunk {
                                        let btn_id = clay_scope.id(cmd.label);
                                        let mut btn_color = Color::u_rgb(2, 6, 23);
                                        if clay_scope.pointer_over(btn_id) {
                                            btn_color = Color::u_rgb(51, 65, 85);
                                            if mouse_pressed {
                                                let mut guard = state.lock().unwrap();
                                                let full_cmd = format!("echo '{}' > {}", cmd.cmd, guard.port);
                                                guard.log_command(full_cmd.clone());
                                                guard.copied_at = Some(std::time::Instant::now());
                                                if let Some(cb) = &mut clipboard {
                                                    let _ = cb.set_text(full_cmd);
                                                }
                                            }
                                        }

                                        let mut btn = Declaration::<Texture2D, ()>::new();
                                        btn.id(btn_id)
                                            .layout().width(grow!()).padding(Padding::all((5.0 * font_scale) as u16)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                                            .background_color(btn_color)
                                            .corner_radius().all(12.0 * font_scale).end();
                                        clay_scope.with(&btn, |clay_scope| {
                                            clay_scope.text(cmd.label, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                                            clay_scope.text(cmd.cmd, clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                                        });
                                    }
                                    if chunk.len() == 1 {
                                        clay_scope.with(&Declaration::<Texture2D, ()>::new().layout().width(grow!()).end(), |_| {});
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
                    .height(fixed!(400.0 * font_scale))
                    .direction(LayoutDirection::TopToBottom)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
                    .child_gap((16.0 * font_scale) as u16)
                    .end();
                clay_scope.with(&mid_col, |clay_scope| {
                    let mut canvas_box = Declaration::<Texture2D, ()>::new();
                    canvas_box.id(clay_scope.id("canvas"))
                        .layout().width(grow!()).height(grow!()).end()
                        .background_color(Color::u_rgb(30, 41, 59))
                        .corner_radius().all(16.0 * font_scale).end();
                    
                    clay_scope.with(&canvas_box, |clay_scope| {
                        // We'll handle custom drawing for the canvas in the raylib render loop
                        // Just reserve the space here
                        let mut label_box = Declaration::<Texture2D, ()>::new();
                        label_box.layout().padding(Padding::all((10.0 * font_scale) as u16)).direction(LayoutDirection::TopToBottom).end();
                        clay_scope.with(&label_box, |clay_scope| {
                            let (x, y) = {
                                let guard = state.lock().unwrap();
                                (guard.v_pos.x, guard.v_pos.y)
                            };
                            let pos_text = arena.push(format!("X: {:.1}  Y: {:.1}", x, y));
                            clay_scope.text(pos_text, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());
                        });

                        let mut trash_btn = Declaration::<Texture2D, ()>::new();
                        trash_btn.id(clay_scope.id("clear_canvas"))
                            .floating()
                                .attach_points(clay_layout::elements::FloatingAttachPointType::RightTop, clay_layout::elements::FloatingAttachPointType::RightTop)
                                .offset(Vector2::new(-16.0 * font_scale, 16.0 * font_scale))
                            .end()
                            .layout().padding(Padding::all((6.0 * font_scale) as u16)).end()
                            .background_color(Color::u_rgb(127, 29, 29))
                            .corner_radius().all(12.0 * font_scale).end();
                        
                        if clay_scope.pointer_over(clay_scope.id("clear_canvas")) && mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            guard.paths.clear();
                        }

                        clay_scope.with(&trash_btn, |clay_scope| {
                            clay_scope.text(ICON_TRASH, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(248, 113, 113)).end());
                        });
                    });
                });

                // Right Column: Controls
                let mut right_col = Declaration::<Texture2D, ()>::new();
                right_col.layout().width(fixed!(300.0 * font_scale)).height(grow!()).direction(LayoutDirection::TopToBottom).child_gap((16.0 * font_scale) as u16).end();
                clay_scope.with(&right_col, |clay_scope| {
                    // Active CLI
                    let mut cli_box = Declaration::<Texture2D, ()>::new();
                    cli_box.layout().width(grow!()).padding(Padding::all((12.0 * font_scale) as u16)).direction(LayoutDirection::TopToBottom).child_gap((8.0 * font_scale) as u16).end()
                        .background_color(Color::u_rgb(0, 0, 0))
                        .corner_radius().all(16.0 * font_scale).end();
                    
                    clay_scope.with(&cli_box, |clay_scope| {
                        let mut label_line = Declaration::<Texture2D, ()>::new();
                        label_line.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&label_line, |clay_scope| {
                            clay_scope.text("Active CLI String", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(59, 130, 246)).end());
                            let copied = {
                                let guard = state.lock().unwrap();
                                guard.copied_at.map(|t| t.elapsed().as_secs_f32() < 2.0).unwrap_or(false)
                            };
                            if copied {
                                clay_scope.text("COPIED", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(74, 222, 128)).end());
                            }
                        });

                        let cmd_text = {
                            let guard = state.lock().unwrap();
                            if guard.last_command.is_empty() { "Ready...".to_string() } else { guard.last_command.clone() }
                        };
                        clay_scope.text(arena.push(cmd_text), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(219, 234, 254)).end());
                    });

                    // Movement
                    let mut move_box = Declaration::<Texture2D, ()>::new();
                    move_box.layout().width(grow!()).padding(Padding::all((12.0 * font_scale) as u16)).direction(LayoutDirection::TopToBottom).child_gap((16.0 * font_scale) as u16).end()
                        .background_color(Color::u_rgb(30, 41, 59))
                        .corner_radius().all(16.0 * font_scale).end();
                    
                    clay_scope.with(&move_box, |clay_scope| {
                        let mut title = Declaration::<Texture2D, ()>::new();
                        title.layout().child_gap((8.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&title, |clay_scope| {
                            clay_scope.text(ICON_MOVE, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(96, 165, 250)).end());
                            clay_scope.text("Movement", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                        });

                        let (dist, feed) = {
                            let guard = state.lock().unwrap();
                            (guard.distance, guard.feed_rate)
                        };

                        render_slider(clay_scope, "dist_slider", "Step (mm)", dist, 0.1, 100.0, Color::u_rgb(59, 130, 246), &state, |s, v| s.distance = v, mouse_pos, mouse_down, &arena, font_scale);
                        render_slider(clay_scope, "feed_slider", "Speed (F)", feed, 10.0, 6000.0, Color::u_rgb(16, 185, 129), &state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, &arena, font_scale);

                        // Jog Controls
                        let mut jog_grid = Declaration::<Texture2D, ()>::new();
                        jog_grid.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).direction(LayoutDirection::TopToBottom).child_gap((8.0 * font_scale) as u16).end();
                        clay_scope.with(&jog_grid, |clay_scope| {
                            // Up
                            let mut row1 = Declaration::<Texture2D, ()>::new(); row1.layout().child_gap((8.0 * font_scale) as u16).end();
                            clay_scope.with(&row1, |clay_scope| {
                                render_jog_btn(clay_scope, "up", ICON_ARROW_UP, &state, "Y", 1.0, mouse_pressed, &mut clipboard, font_scale);
                            });
                            // Left, Cross, Right
                            let mut row2 = Declaration::<Texture2D, ()>::new(); row2.layout().child_gap((8.0 * font_scale) as u16).end();
                            clay_scope.with(&row2, |clay_scope| {
                                render_jog_btn(clay_scope, "left", ICON_ARROW_LEFT, &state, "X", -1.0, mouse_pressed, &mut clipboard, font_scale);
                                
                                let center_id = clay_scope.id("center");
                                let mut center_color = Color::u_rgb(0, 0, 0);
                                if clay_scope.pointer_over(center_id) {
                                    center_color = Color::u_rgb(30, 41, 59);
                                    if mouse_pressed {
                                        let mut guard = state.lock().unwrap();
                                        guard.v_pos = Vector2::new(0.0, 0.0);
                                        let cmd = format!("echo 'G92 X0 Y0' > {}", guard.port);
                                        guard.log_command(cmd.clone());
                                        guard.copied_at = Some(std::time::Instant::now());
                                        if let Some(cb) = &mut clipboard { let _ = cb.set_text(cmd); }
                                    }
                                }
                                let mut center_btn = Declaration::<Texture2D, ()>::new();
                                center_btn.id(center_id).layout().padding(Padding::all((10.0 * font_scale) as u16)).end().background_color(center_color).corner_radius().all(8.0 * font_scale).end();
                                clay_scope.with(&center_btn, |clay_scope| {
                                    clay_scope.text(ICON_CROSSHAIR, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(59, 130, 246)).end());
                                });

                                render_jog_btn(clay_scope, "right", ICON_ARROW_RIGHT, &state, "X", 1.0, mouse_pressed, &mut clipboard, font_scale);
                            });
                            // Down
                            let mut row3 = Declaration::<Texture2D, ()>::new(); row3.layout().child_gap((8.0 * font_scale) as u16).end();
                            clay_scope.with(&row3, |clay_scope| {
                                render_jog_btn(clay_scope, "down", ICON_ARROW_DOWN, &state, "Y", -1.0, mouse_pressed, &mut clipboard, font_scale);
                            });
                        });
                    });

                    // Power Tuning
                    let mut power_box = Declaration::<Texture2D, ()>::new();
                    power_box.layout().width(grow!()).padding(Padding::all((12.0 * font_scale) as u16)).direction(LayoutDirection::TopToBottom).child_gap((16.0 * font_scale) as u16).end()
                        .background_color(Color::u_rgb(30, 41, 59))
                        .corner_radius().all(16.0 * font_scale).end();
                    
                    clay_scope.with(&power_box, |clay_scope| {
                        let mut title = Declaration::<Texture2D, ()>::new();
                        title.layout().child_gap((8.0 * font_scale) as u16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                        clay_scope.with(&title, |clay_scope| {
                            clay_scope.text(ICON_FLAME, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(192, 132, 252)).end());
                            clay_scope.text("Power Tuning", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                        });

                        let pwr = {
                            let guard = state.lock().unwrap();
                            guard.power
                        };
                        render_slider(clay_scope, "power_slider", "Intensity (S)", pwr, 0.0, 1000.0, Color::u_rgb(168, 85, 247), &state, |s, v| s.power = v, mouse_pos, mouse_down, &arena, font_scale);

                        let burn_id = clay_scope.id("burn_btn");
                        let mut burn_color = Color::u_rgb(147, 51, 234); // purple-600
                        if clay_scope.pointer_over(burn_id) {
                            burn_color = Color::u_rgb(168, 85, 247); // purple-500
                            if mouse_pressed {
                                let mut guard = state.lock().unwrap();
                                let d = guard.distance;
                                let f = guard.feed_rate;
                                let s = guard.power;
                                let port = guard.port.clone();
                                let v_pos = guard.v_pos;
                                
                                let new_x = (v_pos.x + d).min(400.0);
                                guard.paths.push(PathSegment { x1: v_pos.x, y1: v_pos.y, x2: new_x, y2: v_pos.y, s });
                                guard.v_pos.x = new_x;
                                
                                let cmd = format!("echo 'G1 X{:.2} F{} S{}' > {}", new_x, f, s, port);
                                guard.log_command(cmd.clone());
                                guard.copied_at = Some(std::time::Instant::now());
                                if let Some(cb) = &mut clipboard { let _ = cb.set_text(cmd); }
                            }
                        }

                        let mut burn_btn = Declaration::<Texture2D, ()>::new();
                        burn_btn.id(burn_id)
                            .layout().width(grow!()).padding(Padding::all((8.0 * font_scale) as u16)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                            .background_color(burn_color)
                            .corner_radius().all(12.0 * font_scale).end();
                        clay_scope.with(&burn_btn, |clay_scope| {
                            clay_scope.text("Burn Segment", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                        });

                        let fire_id = clay_scope.id("fire_btn");
                        let mut fire_color = Color::u_rgb(2, 6, 23);
                        if clay_scope.pointer_over(fire_id) {
                            fire_color = Color::u_rgb(30, 41, 59);
                            if mouse_pressed {
                                let mut guard = state.lock().unwrap();
                                let fire_cmd = if guard.wattage == "10W" { "M3 S5" } else { "M3 S10" };
                                let cmd = format!("echo '{}' > {}", fire_cmd, guard.port);
                                guard.log_command(cmd.clone());
                                guard.copied_at = Some(std::time::Instant::now());
                                if let Some(cb) = &mut clipboard { let _ = cb.set_text(cmd); }
                            }
                        }
                        let mut fire_btn = Declaration::<Texture2D, ()>::new();
                        fire_btn.id(fire_id)
                            .layout().width(grow!()).padding(Padding::all((8.0 * font_scale) as u16)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                            .background_color(fire_color)
                            .corner_radius().all(12.0 * font_scale).end();
                        clay_scope.with(&fire_btn, |clay_scope| {
                            clay_scope.text("Focus Mode Fire", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(192, 132, 252)).end());
                        });
                    });
                });
            });

            // Serial Output
            let mut serial_box = Declaration::<Texture2D, ()>::new();
            serial_box.layout()
                .width(grow!())
                .height(fixed!(150.0 * font_scale))
                .padding(Padding::all((12.0 * font_scale) as u16))
                .direction(LayoutDirection::TopToBottom)
                .child_gap((4.0 * font_scale) as u16)
                .end()
                .background_color(Color::u_rgb(2, 6, 23))
                .corner_radius().all(16.0 * font_scale).end();
            
            clay_scope.with(&serial_box, |clay_scope| {
                clay_scope.text("SERIAL OUTPUT (RECENT COMMANDS)", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                
                let logs = {
                    let guard = state.lock().unwrap();
                    guard.serial_logs.clone()
                };
                
                for log in logs.iter().rev() {
                    clay_scope.text(arena.push(log.clone()), clay_layout::text::TextConfig::new().font_size((11.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                }
            });

            // Footer
            let mut footer_box = Declaration::<Texture2D, ()>::new();
            footer_box.layout().width(grow!()).padding(Padding::all((10.0 * font_scale) as u16)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
            clay_scope.with(&footer_box, |clay_scope| {
                clay_scope.text("Comgrow Z1 Engineering Tool", clay_layout::text::TextConfig::new().font_size((9.0 * font_scale) as u16).color(Color::u_rgb(51, 65, 85)).end());
            });
        });

        let render_commands = clay_scope.end();

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::color::Color::BLACK);
        
        let mut canvas_rect = raylib::math::Rectangle::default();

        for command in render_commands {
            match command.config {
                RenderCommandConfig::Rectangle(rect) => {
                    let r = raylib::math::Rectangle::new(command.bounding_box.x, command.bounding_box.y, command.bounding_box.width, command.bounding_box.height);
                    let target_id = unsafe { clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from("canvas"), 0, 0) };
                    if command.id == target_id.id {
                        canvas_rect = r;
                    }
                    if rect.corner_radii.top_left > 0.0 {
                         d.draw_rectangle_rounded(r, rect.corner_radii.top_left / (command.bounding_box.height / 2.0), 10,
                            raylib::color::Color::new(rect.color.r as u8, rect.color.g as u8, rect.color.b as u8, rect.color.a as u8));
                    } else {
                        d.draw_rectangle(r.x as i32, r.y as i32, r.width as i32, r.height as i32,
                            raylib::color::Color::new(rect.color.r as u8, rect.color.g as u8, rect.color.b as u8, rect.color.a as u8));
                    }
                }
                RenderCommandConfig::Text(text) => {
                    let sanitized = text.text.replace('\0', "");
                    d.draw_text_ex(&font, &sanitized, raylib::math::Vector2::new(command.bounding_box.x, command.bounding_box.y),
                        command.bounding_box.height, 0.0, raylib::color::Color::new(text.color.r as u8, text.color.g as u8, text.color.b as u8, text.color.a as u8));
                }
                _ => {}
            }
        }

        // Draw Canvas Content
        if canvas_rect.width > 0.0 {
            let margin = 20.0 * font_scale;
            let full_draw_width = canvas_rect.width - margin * 2.0;
            let full_draw_height = canvas_rect.height - margin * 2.0;
            let side = full_draw_width.min(full_draw_height);
            
            let offset_x = (full_draw_width - side) / 2.0;
            let offset_y = (full_draw_height - side) / 2.0;

            let draw_area = raylib::math::Rectangle::new(
                canvas_rect.x + margin + offset_x, 
                canvas_rect.y + margin + offset_y, 
                side, 
                side
            );
            
            // Grid
            for i in 0..=10 {
                let x = draw_area.x + (i as f32 / 10.0) * side;
                let y = draw_area.y + (i as f32 / 10.0) * side;
                d.draw_line_v(raylib::math::Vector2::new(x, draw_area.y), raylib::math::Vector2::new(x, draw_area.y + draw_area.height), raylib::color::Color::new(59, 130, 246, 20));
                d.draw_line_v(raylib::math::Vector2::new(draw_area.x, y), raylib::math::Vector2::new(draw_area.x + draw_area.width, y), raylib::color::Color::new(59, 130, 246, 20));
            }

            // Paths
            let guard = state.lock().unwrap();
            for p in &guard.paths {
                let start = raylib::math::Vector2::new(draw_area.x + (p.x1 / 400.0) * side, draw_area.y + draw_area.height - (p.y1 / 400.0) * side);
                let end = raylib::math::Vector2::new(draw_area.x + (p.x2 / 400.0) * side, draw_area.y + draw_area.height - (p.y2 / 400.0) * side);
                d.draw_line_ex(start, end, 2.0, raylib::color::Color::new(255, 71, 87, (p.s / 1000.0 * 255.0) as u8));
            }

            // Laser Head
            let head_pos = raylib::math::Vector2::new(draw_area.x + (guard.v_pos.x / 400.0) * side, draw_area.y + draw_area.height - (guard.v_pos.y / 400.0) * side);
            d.draw_circle_v(head_pos, 5.0 * font_scale, raylib::color::Color::new(59, 130, 246, 100));
            d.draw_circle_v(head_pos, 2.0 * font_scale, raylib::color::Color::RED);
        }
    }

    Ok(())
}

fn render_jog_btn<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    icon: &str,
    state: &Arc<Mutex<AppState>>,
    axis: &str,
    direction: f32,
    mouse_pressed: bool,
    clipboard: &mut Option<Clipboard>,
    font_scale: f32,
) where
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut color = Color::u_rgb(30, 41, 59);
    if clay.pointer_over(btn_id) {
        color = Color::u_rgb(59, 130, 246);
        if mouse_pressed {
            let mut guard = state.lock().unwrap();
            let d = guard.distance;
            if axis == "X" {
                guard.v_pos.x = (guard.v_pos.x + d * direction).clamp(0.0, 400.0);
            } else {
                guard.v_pos.y = (guard.v_pos.y + d * direction).clamp(0.0, 400.0);
            }
            let cmd = format!("echo '$J=G91 G21 {}{} F{}' > {}", axis, direction * d, guard.feed_rate, guard.port);
            guard.log_command(cmd.clone());
            guard.copied_at = Some(std::time::Instant::now());
            if let Some(cb) = clipboard { let _ = cb.set_text(cmd); }
        }
    }
    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id).layout().padding(Padding::all((10.0 * font_scale) as u16)).end().background_color(color).corner_radius().all(8.0 * font_scale).end();
    clay.with(&btn, |clay| {
        clay.text(icon, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
    });
}

fn render_slider<'a, 'render, F>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    color: Color,
    state: &Arc<Mutex<AppState>>,
    update: F,
    _mouse_pos: raylib::math::Vector2,
    mouse_down: bool,
    arena: &StringArena,
    font_scale: f32,
) where
    F: FnOnce(&mut AppState, f32),
    'a: 'render,
{
    let slider_id = clay.id(id);
    let mut container = Declaration::<Texture2D, ()>::new();
    container.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap((4.0 * font_scale) as u16).end();
    
    clay.with(&container, |clay| {
        let mut header = Declaration::<Texture2D, ()>::new();
        header.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
        clay.with(&header, |clay| {
            clay.text(label, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(100, 116, 139)).end());
            clay.text(arena.push(format!("{:.1}", value)), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(color).end());
        });

        let mut track = Declaration::<Texture2D, ()>::new();
        track.id(slider_id).layout().width(grow!()).height(fixed!(6.0 * font_scale)).end()
            .background_color(Color::u_rgb(2, 6, 23))
            .corner_radius().all(3.0 * font_scale).end();
        
        // Clay 0.4.0 doesn't easily expose bounding boxes for elements during layout
        // We can use pointer_over for hover, but for click position we might need another way
        // For now, let's simplify and just use pointer_over + mouse_down to update value
        // based on a fixed assumption of width if we can't get it
        if clay.pointer_over(slider_id) && mouse_down {
             // In a real clay app we'd get the bounding box from the previous frame's render commands
             // or use a custom element. For this port, let's use a simpler heuristic or just toggle
             let mut guard = state.lock().unwrap();
             // Just incrementing for now since we don't have rect here
             let next = if value + (max - min) / 20.0 > max { min } else { value + (max - min) / 20.0 };
             update(&mut guard, next);
        }

        clay.with(&track, |clay| {
            let mut bar = Declaration::<Texture2D, ()>::new();
            let percent = (value - min) / (max - min);
            bar.layout().width(fixed!(percent * 260.0 * font_scale)).height(grow!()).end()
                .background_color(color)
                .corner_radius().all(3.0 * font_scale).end();
            clay.with(&bar, |_| {});
        });
    });
}

