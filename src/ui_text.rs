use crate::cli_and_helpers::generate_text_gcode;
use crate::icons::*;
use crate::state::{AppState, MachineState, StringArena};
use crate::styles::*;
use crate::theme::Theme;
use crate::ui_components::{Section, render_burn_btn, render_checkbox, render_outline_btn, render_slider, Interaction};
use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::{Declaration, fixed, grow};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};

pub fn render_text_controls<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    _sections: &[Section],
    clipboard: &mut Option<Clipboard>,
    arena: &StringArena,
    font_scale: f32,
    theme: &Theme,
    interaction: &mut Interaction,
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
                    if interaction.mouse_pressed {
                        interaction.is_handled = true;
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
                    clipboard,
                    arena,
                    font_scale,
                    !is_idle,
                    theme,
                    interaction,
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
                    font_scale,
                    !is_idle,
                    theme,
                    interaction,
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
                let (font_name, is_open, available_fonts, scroll_offset) = {
                    let g = state.lock().unwrap();
                    (
                        (*g.text_font).clone(),
                        g.text_font_dropdown_open,
                        (*g.available_fonts).clone(),
                        g.text_font_scroll_offset,
                    )
                };

                crate::ui_components::render_dropdown(
                    clay_scope,
                    "font_selector",
                    &font_name,
                    &available_fonts,
                    is_open,
                    scroll_offset,
                    state,
                    arena,
                    font_scale,
                    theme,
                    interaction,
                    |s| s.text_font_dropdown_open = !s.text_font_dropdown_open,
                    |s, val| {
                        s.text_font = Arc::new(val);
                        s.text_font_dropdown_open = false;
                        s.clear_preview();
                    },
                    |s, offset| s.text_font_scroll_offset = offset,
                );
            });

            // Input Box
            crate::ui_components::render_text_input(
                clay_scope,
                "text_input",
                state,
                arena,
                font_scale,
                theme,
                interaction,
            );
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
                    render_slider(clay_scope, "t_pwr", "Power", pwr, 0.0, 1000.0, COLOR_SLIDER_POWER, state, |s, v| { s.power = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    render_slider(clay_scope, "t_spd", "Speed", spd, 10.0, 6000.0, COLOR_SLIDER_SPEED, state, |s, v| { s.feed_rate = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    render_slider(clay_scope, "t_scl", "Scale", scl, 0.1, 10.0, COLOR_SLIDER_STEP, state, |s, v| { s.scale = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    render_slider(clay_scope, "t_pas", "Passes", passes as f32, 1.0, 20.0, COLOR_SLIDER_PASSES, state, |s, v| { s.passes = v as u32; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    render_checkbox(clay_scope, "t_bold", "Bold", bold, state, |s, v| { s.text_is_bold = v; s.clear_preview(); }, font_scale, theme, interaction);
                });

                let mut col2 = Declaration::<Texture2D, ()>::new();
                col2.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(clay_scope, "t_lspc", "Letter Spacing", l_spc, -50.0, 100.0, COLOR_SLIDER_X, state, |s, v| { s.text_letter_spacing = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    render_slider(clay_scope, "t_lispc", "Line Spacing", li_spc, -50.0, 100.0, COLOR_SLIDER_Y, state, |s, v| { s.text_line_spacing = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    render_slider(clay_scope, "t_curv", "Curve Steps", curve as f32, 1.0, 50.0, COLOR_SLIDER_W, state, |s, v| { s.text_curve_steps = v as u32; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    if !outline {
                        render_slider(clay_scope, "t_lpm", "Lines/mm", lpm, 1.0, 20.0, COLOR_SLIDER_PASSES, state, |s, v| { s.text_lines_per_mm = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    }
                    render_checkbox(clay_scope, "t_out", "Outline", outline, state, |s, v| { s.text_is_outline = v; s.clear_preview(); }, font_scale, theme, interaction);
                });
            });

            render_checkbox(clay_scope, "t_ben", "Enable Bounds", b_en, state, |s, v| { s.bounds.enabled = v; s.clear_preview(); }, font_scale, theme, interaction);

            if b_en {
                let mut grid = Declaration::<Texture2D, ()>::new();
                grid.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&grid, |clay_scope| {
                    let mut r1 = Declaration::<Texture2D, ()>::new();
                    r1.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
                    clay_scope.with(&r1, |clay_scope| {
                        render_slider(clay_scope, "t_bx", "X", bx, 0.0, 400.0, COLOR_SLIDER_X, state, |s, v| { s.bounds.x = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                        render_slider(clay_scope, "t_by", "Y", by, 0.0, 400.0, COLOR_SLIDER_Y, state, |s, v| { s.bounds.y = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    });
                    let mut r2 = Declaration::<Texture2D, ()>::new();
                    r2.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
                    clay_scope.with(&r2, |clay_scope| {
                        render_slider(clay_scope, "t_bw", "W", bw, 1.0, 400.0, COLOR_SLIDER_W, state, |s, v| { s.bounds.w = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                        render_slider(clay_scope, "t_bh", "H", bh, 1.0, 400.0, COLOR_SLIDER_H, state, |s, v| { s.bounds.h = v; s.clear_preview(); }, arena, font_scale, theme, interaction);
                    });
                });
            }
        });
    });
}
