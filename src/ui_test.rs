use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::{Declaration, grow};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use crate::state::{AppState, StringArena};
use crate::ui::{Section, render_burn_btn, render_slider, render_checkbox};
use crate::cli_and_helpers::generate_pattern_gcode;
use rfd::FileDialog;
use crate::icons::*;
use crate::styles::*;

pub fn render_test_controls<'a, 'render>(
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
        // 1. Boundary Settings (At the top)
        let mut boundary_box = Declaration::<Texture2D, ()>::new();
        boundary_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(12)).child_gap(12).end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius().all(16.0 * font_scale).end();
        
        clay_scope.with(&boundary_box, |clay_scope| {
            clay_scope.text("BOUNDARY SETTINGS", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(COLOR_TEXT_MUTED).end());
            
            let (enabled, bx, by, bw, bh) = {
                let g = state.lock().unwrap();
                (g.boundary_enabled, g.boundary_x, g.boundary_y, g.boundary_w, g.boundary_h)
            };

            render_checkbox(clay_scope, "boundary_enabled", "Enable Boundary Clipping", enabled, state, |s, v| s.boundary_enabled = v, mouse_pressed, font_scale);
            
            let mut grid = Declaration::<Texture2D, ()>::new();
            grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new(); col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(clay_scope, "bound_x", "X Pos", bx, 0.0, 400.0, COLOR_SLIDER_X, state, |s, v| s.boundary_x = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "bound_w", "Width", bw, 1.0, 400.0, COLOR_SLIDER_W, state, |s, v| s.boundary_w = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
                let mut col2 = Declaration::<Texture2D, ()>::new(); col2.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(clay_scope, "bound_y", "Y Pos", by, 0.0, 400.0, COLOR_SLIDER_Y, state, |s, v| s.boundary_y = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "bound_h", "Height", bh, 1.0, 400.0, COLOR_SLIDER_H, state, |s, v| s.boundary_h = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
            });
        });

        // 2. SVG Controls
        let mut svg_box = Declaration::<Texture2D, ()>::new();
        svg_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(12)).child_gap(12).end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius().all(16.0 * font_scale).end();
        
        clay_scope.with(&svg_box, |clay_scope| {
            clay_scope.text("SVG LOADING", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(COLOR_TEXT_MUTED).end());
            
            let load_id = clay_scope.id("load_svg_btn");
            let mut load_color = if !is_idle { COLOR_BG_DISABLED } else { COLOR_PRIMARY_HOVER };
            if is_idle && clay_scope.pointer_over(load_id) {
                load_color = COLOR_PRIMARY;
                if mouse_pressed {
                    if let Some(path_buf) = FileDialog::new()
                        .add_filter("Scalable Vector Graphics", &["svg"])
                        .pick_file() 
                    {
                        let path_str = path_buf.to_string_lossy().to_string();
                        let (pwr, spd, scl, pas, b_enabled, bx, by, bw, bh) = {
                            let g = state.lock().unwrap();
                            (g.power / 10.0, g.feed_rate / 10.0, g.scale, g.passes, g.boundary_enabled, g.boundary_x, g.boundary_y, g.boundary_w, g.boundary_h)
                        };
                        
                        let fit = if b_enabled { Some(format!("{}x{}", bw, bh)) } else { None };
                        let center = if b_enabled { format!("{},{}", bx + bw/2.0, by + bh/2.0) } else { "200,200".to_string() };

                        match generate_pattern_gcode(
                            &path_str, 
                            &format!("{}%", pwr), 
                            &format!("{}%", spd), 
                            &format!("{}x", scl), 
                            &pas.to_string(),
                            fit,
                            &center
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
            load_btn.id(load_id).layout().padding(Padding::all(10)).direction(LayoutDirection::LeftToRight).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end().background_color(load_color).corner_radius().all(8.0 * font_scale).end();
            
            let load_text_color = if !is_idle { COLOR_TEXT_DISABLED } else { COLOR_TEXT_WHITE };
            clay_scope.with(&load_btn, |clay| {
                clay.text(ICON_FILE, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(load_text_color).end());
                clay.text("LOAD CUSTOM SVG", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(load_text_color).end());
            });
        });

        // 3. Sliders (TROGDOR)
        let mut controls_box = Declaration::<Texture2D, ()>::new();
        controls_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(12)).child_gap(16).end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius().all(16.0 * font_scale).end();
        
        clay_scope.with(&controls_box, |clay_scope| {
            clay_scope.text("TROGDOR", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(COLOR_TEXT_MUTED).end());

            let (pwr, spd, scl, pas) = {
                let g = state.lock().unwrap();
                (g.power, g.feed_rate, g.scale, g.passes)
            };
            
            let mut grid = Declaration::<Texture2D, ()>::new();
            grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new(); col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(clay_scope, "test_power", "Power", pwr, 0.0, 1000.0, COLOR_SLIDER_POWER, state, |s, v| s.power = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "test_scale", "Scale", scl, 0.1, 5.0, COLOR_SLIDER_STEP, state, |s, v| s.scale = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
                let mut col2 = Declaration::<Texture2D, ()>::new(); col2.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(clay_scope, "test_speed", "Speed", spd, 10.0, 6000.0, COLOR_SLIDER_SPEED, state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "test_passes", "Passes", pas as f32, 1.0, 20.0, COLOR_SLIDER_PASSES, state, |s, v| s.passes = v as u32, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
            });
        });

        // 4. Test Patterns (2 Column Layout)
        for section in sections {
            if section.title == "Test Patterns" {
                let mut section_box = Declaration::<Texture2D, ()>::new();
                section_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(16)).child_gap(12).end()
                    .background_color(COLOR_BG_SECTION)
                    .corner_radius().all(16.0 * font_scale).end();
                
                clay_scope.with(&section_box, |clay| {
                    clay.text(section.title, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(section.color).end());
                    
                    for row_chunk in section.commands.chunks(2) {
                        let mut row = Declaration::<Texture2D, ()>::new();
                        row.layout().width(grow!()).child_gap(12).end();
                        clay.with(&row, |clay| {
                            for cmd in row_chunk {
                                let mut btn_row = Declaration::<Texture2D, ()>::new();
                                btn_row.layout().child_gap(4).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                                clay.with(&btn_row, |clay| {
                                    if render_burn_btn(clay, arena.push(format!("test_{}", cmd.label)), cmd.label, state, 0.0, 0.0, mouse_pressed, clipboard, font_scale, !is_idle) {
                                        let (pwr, spd, scl, pas, b_enabled, bx, by, bw, bh) = {
                                            let g = state.lock().unwrap();
                                            (g.power / 10.0, g.feed_rate / 10.0, g.scale, g.passes, g.boundary_enabled, g.boundary_x, g.boundary_y, g.boundary_w, g.boundary_h)
                                        };
                                        
                                        let fit = if b_enabled { Some(format!("{}x{}", bw, bh)) } else { None };
                                        let center = if b_enabled { format!("{},{}", bx + bw/2.0, by + bh/2.0) } else { "200,200".to_string() };

                                        match generate_pattern_gcode(
                                            cmd.label, 
                                            &format!("{}%", pwr), 
                                            &format!("{}%", spd), 
                                            &format!("{}x", scl), 
                                            &pas.to_string(),
                                            fit,
                                            &center 
                                        ) {
                                            Ok((gcode, _)) => {
                                                state.lock().unwrap().send_command(gcode);
                                            }
                                            Err(e) => {
                                                println!("Error generating G-code: {}", e);
                                            }
                                        }
                                    }

                                    // Preview Eyeball
                                    let eye_id = clay.id(arena.push(format!("eye_{}", cmd.label)));
                                    let is_previewing = { state.lock().unwrap().preview_pattern.as_deref() == Some(cmd.label) };
                                    let mut eye_color = if is_previewing { COLOR_SUCCESS } else { COLOR_TEXT_MUTED };
                                    if clay.pointer_over(eye_id) {
                                        eye_color = COLOR_TEXT_WHITE;
                                        if mouse_pressed {
                                            let mut g = state.lock().unwrap();
                                            if is_previewing {
                                                g.preview_pattern = None;
                                                g.preview_paths.clear();
                                            } else {
                                                g.preview_pattern = Some(cmd.label.to_string());
                                                g.preview_paths.clear();
                                                let (pwr, spd, scl, pas, b_enabled, bx, by, bw, bh) = (g.power / 10.0, g.feed_rate / 10.0, g.scale, g.passes, g.boundary_enabled, g.boundary_x, g.boundary_y, g.boundary_w, g.boundary_h);
                                                let fit = if b_enabled { Some(format!("{}x{}", bw, bh)) } else { None };
                                                let center = if b_enabled { format!("{},{}", bx + bw/2.0, by + bh/2.0) } else { "200,200".to_string() };
                                                
                                                // 10x speed for preview
                                                let preview_spd = (spd * 10.0).min(1000.0);

                                                if let Ok((gcode, _)) = generate_pattern_gcode(cmd.label, &format!("{}%", pwr), &format!("{}%", preview_spd), &format!("{}x", scl), &pas.to_string(), fit, &center) {
                                                    let original_v_pos = g.v_pos;
                                                    let original_is_abs = g.is_absolute;
                                                    for line in gcode.lines() {
                                                        g.process_command_for_preview(line);
                                                    }
                                                    g.v_pos = original_v_pos;
                                                    g.is_absolute = original_is_abs;
                                                }
                                            }
                                        }
                                    }
                                    let mut eye_btn = Declaration::<Texture2D, ()>::new();
                                    eye_btn.id(eye_id).layout().padding(Padding::all(4)).end();
                                    clay.with(&eye_btn, |clay| {
                                        clay.text(ICON_EYE, clay_layout::text::TextConfig::new().font_size((20.0 * font_scale) as u16).color(eye_color).end());
                                    });
                                });
                            }
                        });
                    }
                });
            }
        }
    });
}
