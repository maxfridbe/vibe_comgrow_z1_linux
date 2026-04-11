use crate::cli_and_helpers::generate_text_gcode;
use crate::icons::*;
use crate::state::{AppState, MachineState, StringArena};
use crate::styles::*;
use crate::theme::Theme;
use crate::ui_components::{Section, render_burn_btn, render_checkbox, render_outline_btn, render_slider};
use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::math::Vector2 as ClayVector2;
use clay_layout::{Declaration, fixed, grow};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};

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
    theme: &Theme,
) where
    'a: 'render,
{
    let is_idle = state.lock().unwrap().machine_state == MachineState::Idle;
    let is_processing = state.lock().unwrap().is_processing;

    let mut container = Declaration::<Texture2D, ()>::new();
    container.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();

    clay.with(&container, |clay_scope| {
        // 1. Bounds/Burn Row
        let mut bounds_box = Declaration::<Texture2D, ()>::new();
        bounds_box
            .layout()
            .width(grow!())
            .direction(LayoutDirection::TopToBottom)
            .padding(Padding::all(12))
            .child_gap(12)
            .end()
            .background_color(theme.cl_bg_section)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();

        clay_scope.with(&bounds_box, |clay_scope| {
            let mut burn_row = Declaration::<Texture2D, ()>::new();
            burn_row.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(12).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
            clay_scope.with(&burn_row, |clay_scope| {
                let preview_id = clay_scope.id("preview_text");
                let is_active_preview = {
                    let g = state.lock().unwrap();
                    g.preview_pattern.as_ref().map(|p| **p == "text").unwrap_or(false)
                };
                let mut preview_color = if is_active_preview { theme.cl_primary } else { theme.cl_bg_dark };
                if clay_scope.pointer_over(preview_id) {
                    preview_color = theme.cl_primary_hover;
                    if mouse_pressed {
                        let mut g = state.lock().unwrap();
                        if is_active_preview {
                            g.preview_pattern = None;
                            g.preview_paths.clear();
                            g.preview_version += 1;
                        } else {
                            g.preview_pattern = Some(Arc::new("text".to_string()));
                            g.preview_paths.clear();
                            g.preview_version += 1;
                            g.is_processing = true;
                            let config = g.get_text_burn_config();
                            let state_clone = Arc::clone(state);
                            std::thread::spawn(move || {
                                if let Ok((gcode, _)) = generate_text_gcode(&config, true) {
                                    let mut g = state_clone.lock().unwrap();
                                    g.process_command_for_preview(&gcode);
                                    g.is_processing = false;
                                } else {
                                    state_clone.lock().unwrap().is_processing = false;
                                }
                            });
                        }
                    }
                }

                let mut preview_btn = Declaration::<Texture2D, ()>::new();
                preview_btn
                    .id(preview_id)
                    .layout()
                    .width(fixed!(theme.sz_btn_height * font_scale))
                    .height(fixed!(theme.sz_btn_height * font_scale))
                    .direction(LayoutDirection::TopToBottom)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .padding(Padding::all(2))
                    .end()
                    .background_color(preview_color)
                    .corner_radius()
                    .all(8.0 * font_scale)
                    .end();
                clay_scope.with(&preview_btn, |clay| {
                    clay.text(
                        if is_active_preview { ICON_EYE } else { ICON_EYE_SLASH },
                        clay_layout::text::TextConfig::new()
                            .font_size((24.0 * font_scale) as u16)
                            .color(theme.cl_text_white)
                            .end(),
                    );
                });
                if render_burn_btn(
                    clay_scope,
                    "burn_text",
                    "BURN",
                    state,
                    0.0,
                    0.0,
                    mouse_pressed,
                    clipboard,
                    arena,
                    font_scale,
                    !is_idle,
                    theme,
                ) {
                    let state_data = {
                        let mut g = state.lock().unwrap();
                        g.is_burning = true;
                        g.burn_log_active = true;
                        g.is_processing = true;
                        g.get_text_burn_config()
                    };
                    let state_clone = Arc::clone(state);
                    std::thread::spawn(move || {
                        let result = std::panic::catch_unwind(|| {
                            generate_text_gcode(&state_data, false)
                        });
                        match result {
                            Ok(Ok((gcode, _))) => {
                                state_clone.lock().unwrap().send_command(gcode);
                                state_clone.lock().unwrap().is_processing = false;
                            }
                            _ => {
                                state_clone.lock().unwrap().is_processing = false;
                            }
                        }
                    });
                }

                render_outline_btn(
                    clay_scope,
                    "outline_text",
                    state,
                    || {
                        let config = state.lock().unwrap().get_text_burn_config();
                        generate_text_gcode(&config, false).ok().map(|(g, _)| g)
                    },
                    mouse_pressed,
                    font_scale,
                    !is_idle,
                    theme,
                );
            });
        });

        // 2. Input Section
        let mut text_box = Declaration::<Texture2D, ()>::new();
        text_box
            .layout()
            .width(grow!())
            .direction(LayoutDirection::TopToBottom)
            .padding(Padding::all(12))
            .child_gap(12)
            .end()
            .background_color(theme.cl_bg_section)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();

        clay_scope.with(&text_box, |clay_scope| {
            clay_scope.text(
                "TEXT CONTENT",
                clay_layout::text::TextConfig::new()
                    .font_size((12.0 * font_scale) as u16)
                    .color(theme.cl_text_sub)
                    .end(),
            );

            // Font Selector
            let mut font_row = Declaration::<Texture2D, ()>::new();
            font_row.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(8).end();
            clay_scope.with(&font_row, |clay_scope| {
                let dropdown_id = clay_scope.id("font_selector");
                let is_open = state.lock().unwrap().text_font_dropdown_open;
                let mut dropdown_color = if is_open {
                    theme.cl_primary
                } else {
                    theme.cl_bg_dark
                };
                if clay_scope.pointer_over(dropdown_id) {
                    dropdown_color = theme.cl_primary_hover;
                    if mouse_pressed {
                        let mut g = state.lock().unwrap();
                        g.text_font_dropdown_open = !g.text_font_dropdown_open;
                    }
                }

                let mut dropdown_btn = Declaration::<Texture2D, ()>::new();
                dropdown_btn
                    .id(dropdown_id)
                    .layout()
                    .width(grow!())
                    .padding(Padding::all(10))
                    .end()
                    .background_color(dropdown_color)
                    .corner_radius()
                    .all(8.0 * font_scale)
                    .end();

                let font_name = state.lock().unwrap().text_font.clone();
                clay_scope.with(&dropdown_btn, |clay| {
                    clay.text(
                        arena.push(format!("{}   {}", ICON_FONT, font_name)),
                        clay_layout::text::TextConfig::new()
                            .font_size((14.0 * font_scale) as u16)
                            .color(theme.cl_text_main)
                            .end(),
                    );
                });
            });

            // Restored Dropdown List Rendering
            let (dropdown_open, available_fonts, scroll_offset) = {
                let g = state.lock().unwrap();
                (g.text_font_dropdown_open, g.available_fonts.clone(), g.text_font_scroll_offset)
            };

            if dropdown_open {
                let mut dropdown_list = Declaration::<Texture2D, ()>::new();
                let dropdown_list_id = clay_scope.id("font_dropdown_list");
                dropdown_list
                    .id(dropdown_list_id)
                    .layout()
                    .width(grow!())
                    .height(fixed!(200.0 * font_scale))
                    .direction(LayoutDirection::TopToBottom)
                    .end()
                    .background_color(theme.cl_bg_dark)
                    .corner_radius()
                    .all(4.0 * font_scale)
                    .end()
                    .clip(
                        false,
                        true,
                        ClayVector2 {
                            x: 0.0,
                            y: scroll_offset,
                        },
                    );

                if clay_scope.pointer_over(dropdown_list_id) {
                    let mut g = state.lock().unwrap();
                    g.text_font_scroll_offset += scroll_y * 40.0;
                    if g.text_font_scroll_offset > 0.0 {
                        g.text_font_scroll_offset = 0.0;
                    }
                    let fonts_count = available_fonts.len();
                    let max_scroll = -((fonts_count as f32 * 32.0 * font_scale) - (200.0 * font_scale)).max(0.0);
                    if g.text_font_scroll_offset < max_scroll {
                        g.text_font_scroll_offset = max_scroll;
                    }
                }

                clay_scope.with(&dropdown_list, |clay_scope| {
                    for font in available_fonts.iter() {
                        let item_id = clay_scope.id(arena.push(format!("font_item_{}", font)));
                        let mut item_color = theme.cl_bg_dark;
                        if clay_scope.pointer_over(item_id) {
                            item_color = theme.cl_bg_section;
                            if mouse_pressed {
                                let mut g = state.lock().unwrap();
                                g.text_font = Arc::new(font.clone());
                                g.text_font_dropdown_open = false;
                            }
                        }
                        let mut item_box = Declaration::<Texture2D, ()>::new();
                        item_box
                            .id(item_id)
                            .layout()
                            .width(grow!())
                            .padding(Padding::all(8))
                            .end()
                            .background_color(item_color);
                        clay_scope.with(&item_box, |clay_scope| {
                            clay_scope.text(
                                arena.push(font.clone()),
                                clay_layout::text::TextConfig::new()
                                    .font_size((12.0 * font_scale) as u16)
                                    .color(theme.cl_text_main)
                                    .end(),
                            );
                        });
                    }
                });
            }

            // Input Box
            let input_id = clay_scope.id("text_input");
            let is_active = state.lock().unwrap().is_text_input_active;
            let mut input_color = theme.cl_bg_dark;
            let border_color = if is_active {
                theme.cl_primary
            } else {
                theme.cl_bg_section
            };

            if clay_scope.pointer_over(input_id) {
                if !is_active {
                    input_color = theme.cl_bg_section;
                }
                if mouse_pressed {
                    let mut g = state.lock().unwrap();
                    g.is_text_input_active = true;
                    g.text_font_dropdown_open = false; // Close font list if typing
                }
            }

            let mut input_box_decl = Declaration::<Texture2D, ()>::new();
            input_box_decl
                .id(input_id)
                .layout()
                .width(grow!())
                .height(fixed!(100.0 * font_scale))
                .padding(Padding::all(12))
                .end()
                .background_color(input_color)
                .corner_radius()
                .all(8.0 * font_scale)
                .end()
                .border()
                .top((2.0 * font_scale) as u16)
                .bottom((2.0 * font_scale) as u16)
                .left((2.0 * font_scale) as u16)
                .right((2.0 * font_scale) as u16)
                .color(border_color)
                .end();

            let (content, cursor_idx) = {
                let g = state.lock().unwrap();
                (g.text_content.clone(), g.text_cursor_index)
            };
            clay_scope.with(&input_box_decl, |clay| {
                let display_text = if content.is_empty() && !is_active {
                    "Type here..."
                } else {
                    &content
                };
                let display_color = if content.is_empty() && !is_active {
                    theme.cl_text_sub
                } else {
                    theme.cl_text_main
                };

                let mut text_arena = display_text.to_string();
                if is_active && (unsafe { raylib::ffi::GetTime() } * 2.0) as i32 % 2 == 0 {
                    let cursor = cursor_idx.min(text_arena.len());
                    text_arena.insert(cursor, '|');
                }

                clay.text(
                    arena.push(text_arena),
                    clay_layout::text::TextConfig::new()
                        .font_size((16.0 * font_scale) as u16)
                        .color(display_color)
                        .end(),
                );
            });
        });

        // 3. Settings Section
        let mut controls_box = Declaration::<Texture2D, ()>::new();
        controls_box
            .layout()
            .width(grow!())
            .direction(LayoutDirection::TopToBottom)
            .padding(Padding::all(12))
            .child_gap(16)
            .end()
            .background_color(theme.cl_bg_section)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();

        clay_scope.with(&controls_box, |clay_scope| {
            clay_scope.text(
                "TEXT SETTINGS",
                clay_layout::text::TextConfig::new()
                    .font_size((12.0 * font_scale) as u16)
                    .color(theme.cl_text_sub)
                    .end(),
            );

            let (pwr, spd, scl, passes, bold, outline, l_spc, li_spc, curve, lpm, b_en, bx, by, bw, bh) = {
                let g = state.lock().unwrap();
                (
                    g.power,
                    g.feed_rate,
                    g.scale,
                    g.passes,
                    g.text_is_bold,
                    g.text_is_outline,
                    g.text_letter_spacing,
                    g.text_line_spacing,
                    g.text_curve_steps,
                    g.text_lines_per_mm,
                    g.bounds.enabled,
                    g.bounds.x,
                    g.bounds.y,
                    g.bounds.w,
                    g.bounds.h,
                )
            };

            let mut settings_grid = Declaration::<Texture2D, ()>::new();
            settings_grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&settings_grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new();
                col1.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(clay_scope, "t_pwr", "Power", pwr, 0.0, 1000.0, COLOR_SLIDER_POWER, state, |s, v| s.power = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    render_slider(clay_scope, "t_spd", "Speed", spd, 10.0, 6000.0, COLOR_SLIDER_SPEED, state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    render_slider(clay_scope, "t_scl", "Scale", scl, 0.1, 10.0, COLOR_SLIDER_STEP, state, |s, v| s.scale = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    render_slider(clay_scope, "t_pas", "Passes", passes as f32, 1.0, 20.0, COLOR_SLIDER_PASSES, state, |s, v| s.passes = v as u32, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    render_checkbox(clay_scope, "t_bold", "Bold", bold, state, |s, v| s.text_is_bold = v, mouse_pressed, font_scale, theme);
                });

                let mut col2 = Declaration::<Texture2D, ()>::new();
                col2.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(clay_scope, "t_lspc", "Letter Spacing", l_spc, -50.0, 100.0, COLOR_SLIDER_X, state, |s, v| s.text_letter_spacing = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    render_slider(clay_scope, "t_lispc", "Line Spacing", li_spc, -50.0, 100.0, COLOR_SLIDER_Y, state, |s, v| s.text_line_spacing = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    render_slider(clay_scope, "t_curv", "Curve Steps", curve as f32, 1.0, 50.0, COLOR_SLIDER_W, state, |s, v| s.text_curve_steps = v as u32, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    if !outline {
                        render_slider(clay_scope, "t_lpm", "Lines/mm", lpm, 1.0, 20.0, COLOR_SLIDER_PASSES, state, |s, v| s.text_lines_per_mm = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    }
                    render_checkbox(clay_scope, "t_out", "Outline", outline, state, |s, v| s.text_is_outline = v, mouse_pressed, font_scale, theme);
                });
            });

            render_checkbox(clay_scope, "t_ben", "Enable Bounds", b_en, state, |s, v| s.bounds.enabled = v, mouse_pressed, font_scale, theme);

            if b_en {
                let mut grid = Declaration::<Texture2D, ()>::new();
                grid.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&grid, |clay_scope| {
                    let mut r1 = Declaration::<Texture2D, ()>::new();
                    r1.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
                    clay_scope.with(&r1, |clay_scope| {
                        render_slider(clay_scope, "t_bx", "X", bx, 0.0, 400.0, COLOR_SLIDER_X, state, |s, v| s.bounds.x = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                        render_slider(clay_scope, "t_by", "Y", by, 0.0, 400.0, COLOR_SLIDER_Y, state, |s, v| s.bounds.y = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    });
                    let mut r2 = Declaration::<Texture2D, ()>::new();
                    r2.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
                    clay_scope.with(&r2, |clay_scope| {
                        render_slider(clay_scope, "t_bw", "W", bw, 1.0, 400.0, COLOR_SLIDER_W, state, |s, v| s.bounds.w = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                        render_slider(clay_scope, "t_bh", "H", bh, 1.0, 400.0, COLOR_SLIDER_H, state, |s, v| s.bounds.h = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    });
                });
            }
        });
    });
}
