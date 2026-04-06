use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::{Declaration, Color, grow, fixed};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use crate::state::{AppState, StringArena};
use crate::ui::{Section, render_burn_btn, render_slider};
use crate::cli_and_helpers::generate_pattern_gcode;
use rfd::FileDialog;

pub fn render_test_left_col<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    sections: &[Section],
    mouse_pos: raylib::math::Vector2,
    mouse_down: bool,
    mouse_pressed: bool,
    scroll_y: f32,
    clipboard: &mut Option<Clipboard>,
    arena: &StringArena,
    font_scale: f32,
) where 'a: 'render {
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();
    
    let is_idle = { state.lock().unwrap().machine_state == "Idle" };

    clay.with(&left_col, |clay_scope| {
        // 1. SVG Controls (At the top)
        let mut svg_box = Declaration::<Texture2D, ()>::new();
        svg_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(12)).child_gap(12).end()
            .background_color(Color::u_rgb(30, 41, 59))
            .corner_radius().all(16.0 * font_scale).end();
        
        clay_scope.with(&svg_box, |clay_scope| {
            clay_scope.text("SVG LOADING", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
            
            let load_id = clay_scope.id("load_svg_btn");
            let mut load_color = if !is_idle { Color::u_rgb(15, 23, 42) } else { Color::u_rgb(37, 99, 235) };
            if is_idle && clay_scope.pointer_over(load_id) {
                load_color = Color::u_rgb(59, 130, 246);
                if mouse_pressed {
                    // RFD Dialog
                    if let Some(path_buf) = FileDialog::new()
                        .add_filter("Scalable Vector Graphics", &["svg"])
                        .pick_file() 
                    {
                        let path_str = path_buf.to_string_lossy().to_string();
                        let (pwr, spd, scl, pas) = {
                            let g = state.lock().unwrap();
                            (g.power / 10.0, g.feed_rate / 10.0, g.scale, g.passes)
                        };
                        
                        match generate_pattern_gcode(
                            &path_str, 
                            &format!("{}%", pwr), 
                            &format!("{}%", spd), 
                            &format!("{}x", scl), 
                            &pas.to_string(),
                            None,
                            "200,200"
                        ) {
                            Ok((gcode, _)) => {
                                state.lock().unwrap().send_command(gcode);
                            }
                            Err(e) => { println!("Error loading SVG: {}", e); }
                        }
                    }
                }
            }
            
            let mut load_btn = Declaration::<Texture2D, ()>::new();
            load_btn.id(load_id).layout().padding(Padding::all(10)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end().background_color(load_color).corner_radius().all(8.0 * font_scale).end();
            
            let load_text_color = if !is_idle { Color::u_rgb(71, 85, 105) } else { Color::u_rgb(255, 255, 255) };
            clay_scope.with(&load_btn, |clay| {
                clay.text("LOAD CUSTOM SVG", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(load_text_color).end());
            });
        });

        // 2. Sliders
        let mut controls_box = Declaration::<Texture2D, ()>::new();
        controls_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(12)).child_gap(16).end()
            .background_color(Color::u_rgb(30, 41, 59))
            .corner_radius().all(16.0 * font_scale).end();
        
        clay_scope.with(&controls_box, |clay_scope| {
            let (pwr, spd, scl, pas) = {
                let g = state.lock().unwrap();
                (g.power, g.feed_rate, g.scale, g.passes)
            };
            
            render_slider(clay_scope, "test_power", "Power", pwr, 0.0, 1000.0, Color::u_rgb(248, 113, 113), state, |s, v| s.power = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
            render_slider(clay_scope, "test_speed", "Speed", spd, 10.0, 6000.0, Color::u_rgb(16, 185, 129), state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
            render_slider(clay_scope, "test_scale", "Scale", scl, 0.1, 5.0, Color::u_rgb(59, 130, 246), state, |s, v| s.scale = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
            render_slider(clay_scope, "test_passes", "Passes", pas as f32, 1.0, 20.0, Color::u_rgb(192, 132, 252), state, |s, v| s.passes = v as u32, mouse_pos, mouse_down, scroll_y, arena, font_scale);
        });

        // 3. Test Patterns
        for section in sections {
            if section.title == "Test Patterns" {
                let mut section_box = Declaration::<Texture2D, ()>::new();
                section_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(16)).child_gap(12).end()
                    .background_color(Color::u_rgb(30, 41, 59))
                    .corner_radius().all(16.0 * font_scale).end();
                
                clay_scope.with(&section_box, |clay| {
                    clay.text(section.title, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(section.color).end());
                    for cmd in &section.commands {
                        if crate::ui::render_burn_btn(clay, arena.push(format!("test_{}", cmd.label)), cmd.label, state, 0.0, 0.0, mouse_pressed, clipboard, font_scale, !is_idle) {
                            let (pwr, spd, scl, pas) = {
                                let g = state.lock().unwrap();
                                (g.power / 10.0, g.feed_rate / 10.0, g.scale, g.passes)
                            };
                            
                            match generate_pattern_gcode(
                                cmd.label, 
                                &format!("{}%", pwr), 
                                &format!("{}%", spd), 
                                &format!("{}x", scl), 
                                &pas.to_string(),
                                None,
                                "200,200" 
                            ) {
                                Ok((gcode, _)) => {
                                    state.lock().unwrap().send_command(gcode);
                                }
                                Err(e) => {
                                    println!("Error generating G-code: {}", e);
                                }
                            }
                        }
                    }
                });
            }
        }
    });
}
