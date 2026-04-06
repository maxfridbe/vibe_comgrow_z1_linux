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
mod styles;

use styles::*;
use clay_layout::layout::{Padding, LayoutDirection, Alignment, LayoutAlignmentX, LayoutAlignmentY};
use clay_layout::math::{Dimensions, Vector2 as ClayVector2};
use clay_layout::{Clay, Declaration, grow, fixed};
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
        last_command: String::new(),
        copied_at: None,
        serial_logs: std::collections::VecDeque::new(),
        tx: mpsc::channel().0,
        boundary_enabled: false,
        boundary_x: 0.0,
        boundary_y: 0.0,
        boundary_w: 100.0,
        boundary_h: 100.0,
    }));

    let (tx, rx) = mpsc::channel();
    state.lock().unwrap().tx = tx;
    start_serial_thread(Arc::clone(&state), rx);

    let (mut rl, thread) = raylib::init()
        .size(1280, 800)
        .title("Comgrow Z1 Laser GRBL Runner")
        .vsync()
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

    let mut clay = Clay::new(Dimensions::new(1280.0, 800.0));
    clay.set_measure_text_function(|text, config| {
        let size = config.font_size as f32;
        let width = text.len() as f32 * (size * 0.60);
        Dimensions::new(width, size)
    });
    let arena = StringArena::new();
    let mut clipboard = Clipboard::new().ok();
    let mut zoom_size: i32 = 64; 
    let mut font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, zoom_size, Some(&font_chars))
        .expect("Failed to load font");

    while !rl.window_should_close() {
        arena.clear();

        if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL) {
            if rl.is_key_pressed(KeyboardKey::KEY_EQUAL) {
                zoom_size = (zoom_size + 16).min(128);
                font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, zoom_size, Some(&font_chars)).expect("Failed to load font");
            }
            if rl.is_key_pressed(KeyboardKey::KEY_MINUS) {
                zoom_size = (zoom_size - 16).max(32);
                font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, zoom_size, Some(&font_chars)).expect("Failed to load font");
            }
        }
        
        let header_font_size = (zoom_size + 6) as u16;
        let font_scale = zoom_size as f32 / 64.0;
        let header_font_size = (zoom_size + 6) as u16;
        let base_font_size = zoom_size as u16;

        let mouse_pos = rl.get_mouse_position();
        let mouse_down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let mouse_pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let scroll_delta = rl.get_mouse_wheel_move_v();
        
        let render_width = rl.get_render_width() as f32;
        let render_height = rl.get_render_height() as f32;

        clay.set_layout_dimensions(Dimensions::new(render_width, render_height));
        clay.pointer_state(clay_layout::math::Vector2 { x: mouse_pos.x, y: mouse_pos.y }, mouse_down);
        clay.update_scroll_containers(true, clay_layout::math::Vector2 { x: scroll_delta.x * 50.0, y: scroll_delta.y * 50.0 }, rl.get_frame_time());

        let mut clay_scope = clay.begin::<Texture2D, ()>();
        
        let mut root_decl = Declaration::<Texture2D, ()>::new();
        root_decl.id(clay_scope.id("root")).layout().width(grow!()).height(grow!()).padding(Padding::all(6)).child_gap(12).direction(LayoutDirection::TopToBottom).end()
            .background_color(COLOR_BG_MAIN);

        clay_scope.with(&root_decl, |clay_scope| {
            let bottom_bar_height = 160.0 * font_scale;
            let standard_margin = (20.0 * font_scale) as u16;

            let mut header_decl = Declaration::<Texture2D, ()>::new();
            header_decl.layout().width(grow!()).height(fixed!(80.0 * font_scale)).padding(Padding::all(12)).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end()
                .background_color(COLOR_BG_SECTION)
                .corner_radius().all(16.0 * font_scale).end();

            clay_scope.with(&header_decl, |clay_scope| {
                let mut title_group = Declaration::<Texture2D, ()>::new();
                title_group.layout().child_gap(16).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                clay_scope.with(&title_group, |clay_scope| {
                    let mut icon_box = Declaration::<Texture2D, ()>::new();
                    icon_box.layout().padding(Padding::all(8)).end().background_color(COLOR_PRIMARY_HOVER).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&icon_box, |clay_scope| {
                        clay_scope.text(ICON_LASER, clay_layout::text::TextConfig::new().font_size(header_font_size).color(COLOR_TEXT_WHITE).end());
                    });
                    clay_scope.text("Comgrow Z1 Laser GRBL Runner", clay_layout::text::TextConfig::new().font_size(header_font_size).color(COLOR_TEXT_WHITE).end());
                });

                let mut spacer = Declaration::<Texture2D, ()>::new(); spacer.layout().width(grow!()).end(); clay_scope.with(&spacer, |_| {});

                let mut settings_group = Declaration::<Texture2D, ()>::new();
                settings_group.layout().child_gap(12).child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Center)).end();
                clay_scope.with(&settings_group, |clay_scope| {
                    let (port, wattage) = { let g = state.lock().unwrap(); (g.port.clone(), g.wattage.clone()) };
                    let mut input_box = Declaration::<Texture2D, ()>::new();
                    input_box.layout().padding(Padding::all(6)).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end().background_color(COLOR_BG_DARK).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&input_box, |clay_scope| {
                        clay_scope.text(ICON_USB, clay_layout::text::TextConfig::new().font_size(header_font_size).color(COLOR_USB_ICON).end());
                        clay_scope.text(arena.push(port), clay_layout::text::TextConfig::new().font_size(header_font_size).color(COLOR_PORT_TEXT).end());
                    });
                    let mut wattage_box = Declaration::<Texture2D, ()>::new();
                    wattage_box.layout().padding(Padding::all(6)).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end().background_color(COLOR_BG_DARK).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&wattage_box, |clay_scope| {
                        clay_scope.text(ICON_CPU, clay_layout::text::TextConfig::new().font_size(header_font_size).color(COLOR_CPU_ICON).end());
                        clay_scope.text(arena.push(wattage), clay_layout::text::TextConfig::new().font_size(header_font_size).color(COLOR_WATTAGE_TEXT).end());
                    });
                });
            });

            // Rest of the UI...
        });

        let render_commands = clay_scope.end();
        // ... rest of the main loop (rendering code)
    }
    Ok(())
}
