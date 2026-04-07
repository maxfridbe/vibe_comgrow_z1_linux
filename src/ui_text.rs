use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::{Declaration, grow, fixed};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use crate::state::{AppState, StringArena};
use crate::ui::{Section, render_burn_btn, render_slider, render_checkbox};
use crate::cli_and_helpers::generate_text_gcode;
use crate::icons::*;
use crate::styles::*;

pub fn render_text_controls<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    _sections: &[Section],
    mouse_pos: raylib::math::Vector2,
    mouse_down: bool,
    mouse_pressed: bool,
    scroll_y: f32,
    clipboard: &mut Option<Clipboard>,
    arena: &StringArena,
    font_scale: f32,
) where 'a: 'render {
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().width(grow!()).height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();
    
    let is_idle = { state.lock().unwrap().machine_state == "Idle" };

    clay.with(&left_col, |clay_scope| {
        // 1. Boundary Settings
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

            render_checkbox(clay_scope, "txt_boundary_enabled", "Enable Boundary Clipping", enabled, state, |s, v| s.boundary_enabled = v, mouse_pressed, font_scale);
            
            let mut grid = Declaration::<Texture2D, ()>::new();
            grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new(); col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(clay_scope, "txt_bound_x", "X Pos", bx, 0.0, 400.0, COLOR_SLIDER_X, state, |s, v| s.boundary_x = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "txt_bound_w", "Width", bw, 1.0, 400.0, COLOR_SLIDER_W, state, |s, v| s.boundary_w = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
                let mut col2 = Declaration::<Texture2D, ()>::new(); col2.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(clay_scope, "txt_bound_y", "Y Pos", by, 0.0, 400.0, COLOR_SLIDER_Y, state, |s, v| s.boundary_y = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "txt_bound_h", "Height", bh, 1.0, 400.0, COLOR_SLIDER_H, state, |s, v| s.boundary_h = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
            });
        });

        // 2. Text Options
        let mut text_box = Declaration::<Texture2D, ()>::new();
        text_box.layout().width(grow!()).direction(LayoutDirection::TopToBottom).padding(Padding::all(12)).child_gap(12).end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius().all(16.0 * font_scale).end();
        
        clay_scope.with(&text_box, |clay_scope| {
            clay_scope.text("TEXT OPTIONS", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(COLOR_TEXT_MUTED).end());
            
            let (content, font_name, is_bold, is_outline, l_space, line_space) = {
                let g = state.lock().unwrap();
                (g.text_content.clone(), g.text_font.clone(), g.text_is_bold, g.text_is_outline, g.text_letter_spacing, g.text_line_spacing)
            };

            // Text content (Simplified textbox for now - in a real app would use raylib input)
            let mut input_row = Declaration::<Texture2D, ()>::new();
            input_row.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
            clay_scope.with(&input_row, |clay_scope| {
                clay_scope.text("Content:", clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(COLOR_TEXT_LABEL).end());
                // Dummy textbox visual
                let mut box_decl = Declaration::<Texture2D, ()>::new();
                box_decl.layout().width(grow!()).padding(Padding::all(8)).end().background_color(COLOR_BG_DARK).corner_radius().all(4.0 * font_scale).end();
                clay_scope.with(&box_decl, |clay_scope| {
                    clay_scope.text(arena.push(content.clone()), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(COLOR_TEXT_WHITE).end());
                });
            });

            let mut style_row = Declaration::<Texture2D, ()>::new();
            style_row.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&style_row, |clay_scope| {
                render_checkbox(clay_scope, "txt_bold", "Bold", is_bold, state, |s, v| s.text_is_bold = v, mouse_pressed, font_scale);
                render_checkbox(clay_scope, "txt_outline", "Outline", is_outline, state, |s, v| s.text_is_outline = v, mouse_pressed, font_scale);
            });

            let mut spacing_grid = Declaration::<Texture2D, ()>::new();
            spacing_grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&spacing_grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new(); col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(clay_scope, "txt_lspace", "Letter Spacing", l_space, -5.0, 20.0, COLOR_SLIDER_X, state, |s, v| s.text_letter_spacing = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
                let mut col2 = Declaration::<Texture2D, ()>::new(); col2.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(clay_scope, "txt_linespace", "Line Spacing", line_space, 0.5, 3.0, COLOR_SLIDER_Y, state, |s, v| s.text_line_spacing = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
            });

            let mut action_row = Declaration::<Texture2D, ()>::new();
            action_row.layout().child_gap(12).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
            clay_scope.with(&action_row, |clay_scope| {
                // Preview Eyeball
                let eye_id = clay_scope.id("eye_text");
                let is_previewing = { state.lock().unwrap().preview_pattern.as_deref() == Some("text") };
                let mut eye_color = if is_previewing { COLOR_SUCCESS } else { COLOR_TEXT_MUTED };
                if clay_scope.pointer_over(eye_id) {
                    eye_color = COLOR_TEXT_WHITE;
                    if mouse_pressed {
                        let mut g = state.lock().unwrap();
                        if is_previewing {
                            g.preview_pattern = None;
                            g.preview_paths.clear();
                        } else {
                            g.preview_pattern = Some("text".to_string());
                            g.preview_paths.clear();
                            g.is_processing = true;
                            let state_data = (g.text_content.clone(), g.power, g.feed_rate, g.scale, g.passes, g.boundary_enabled, g.boundary_x, g.boundary_y, g.boundary_w, g.boundary_h, g.text_is_bold, g.text_is_outline, g.text_letter_spacing, g.text_line_spacing);
                            let state_clone = Arc::clone(state);
                            std::thread::spawn(move || {
                                let (txt, pwr, spd, scl, pas, b_enabled, bx, by, bw, bh, bold, outline, l_space, line_space) = state_data;
                                let fit = if b_enabled { Some((bw, bh)) } else { None };
                                let center = if b_enabled { (bx + bw/2.0, by + bh/2.0) } else { (200.0, 200.0) };
                                
                                if let Ok((gcode, _)) = generate_text_gcode(&txt, pwr, spd * 10.0, scl, pas, fit, center, bold, outline, l_space, line_space) {
                                    let mut g = state_clone.lock().unwrap();
                                    let original_v_pos = g.v_pos;
                                    let original_is_abs = g.is_absolute;
                                    for line in gcode.lines() { g.process_command_for_preview(line); }
                                    g.v_pos = original_v_pos;
                                    g.is_absolute = original_is_abs;
                                }
                                state_clone.lock().unwrap().is_processing = false;
                            });
                        }
                    }
                }
                let mut eye_btn = Declaration::<Texture2D, ()>::new();
                eye_btn.id(eye_id).layout().padding(Padding::all(4)).end();
                clay_scope.with(&eye_btn, |clay| {
                    clay.text(ICON_EYE, clay_layout::text::TextConfig::new().font_size((20.0 * font_scale) as u16).color(eye_color).end());
                });

                if render_burn_btn(clay_scope, "burn_text", "BURN", state, 0.0, 0.0, mouse_pressed, clipboard, font_scale, !is_idle) {
                    let state_data = {
                        let mut g = state.lock().unwrap();
                        g.is_processing = true;
                        (g.text_content.clone(), g.power, g.feed_rate, g.scale, g.passes, g.boundary_enabled, g.boundary_x, g.boundary_y, g.boundary_w, g.boundary_h, g.text_is_bold, g.text_is_outline, g.text_letter_spacing, g.text_line_spacing)
                    };
                    let state_clone = Arc::clone(state);
                    std::thread::spawn(move || {
                        let (txt, pwr, spd, scl, pas, b_enabled, bx, by, bw, bh, bold, outline, l_space, line_space) = state_data;
                        let fit = if b_enabled { Some((bw, bh)) } else { None };
                        let center = if b_enabled { (bx + bw/2.0, by + bh/2.0) } else { (200.0, 200.0) };
                        
                        if let Ok((gcode, _)) = generate_text_gcode(&txt, pwr, spd, scl, pas, fit, center, bold, outline, l_space, line_space) {
                            state_clone.lock().unwrap().send_command(gcode);
                        }
                        state_clone.lock().unwrap().is_processing = false;
                    });
                }
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
                    render_slider(clay_scope, "txt_power", "Power", pwr, 0.0, 1000.0, COLOR_SLIDER_POWER, state, |s, v| s.power = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "txt_scale", "Scale", scl, 0.1, 5.0, COLOR_SLIDER_STEP, state, |s, v| s.scale = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
                let mut col2 = Declaration::<Texture2D, ()>::new(); col2.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(clay_scope, "txt_speed", "Speed", spd, 10.0, 6000.0, COLOR_SLIDER_SPEED, state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                    render_slider(clay_scope, "txt_passes", "Passes", pas as f32, 1.0, 20.0, COLOR_SLIDER_PASSES, state, |s, v| s.passes = v as u32, mouse_pos, mouse_down, scroll_y, arena, font_scale);
                });
            });
        });
    });
}
