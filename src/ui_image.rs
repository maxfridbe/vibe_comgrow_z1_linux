use crate::cli_and_helpers::{generate_image_gcode, get_image_outline_gcode};
use crate::icons::*;
use crate::state::{AppState, MachineState, StringArena, ToastType};
use crate::styles::*;
use crate::theme::Theme;
use crate::ui_components::{Section, render_burn_btn, render_checkbox, render_outline_btn, render_slider};
use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::{Declaration, fixed, grow};
use rfd::FileDialog;
use raylib::prelude::*;
use std::sync::{Arc, Mutex};

pub fn render_image_controls<'a, 'render>(
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
        // 1. Image Picker Section
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
            clay_scope.text(
                "CUSTOM IMAGE",
                clay_layout::text::TextConfig::new()
                    .font_size((12.0 * font_scale) as u16)
                    .color(theme.cl_text_sub)
                    .end(),
            );

            let mut pick_row = Declaration::<Texture2D, ()>::new();
            pick_row
                .layout()
                .width(grow!())
                .direction(LayoutDirection::LeftToRight)
                .child_gap(12)
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                .end();

            clay_scope.with(&pick_row, |clay_scope| {
                let load_id = clay_scope.id("pick_image_btn");
                let mut load_color = if !is_idle {
                    COLOR_BG_DISABLED
                } else {
                    theme.cl_primary_hover
                };
                if is_idle && clay_scope.pointer_over(load_id) {
                    load_color = theme.cl_primary;
                    if mouse_pressed {
                        if let Some(path_buf) =
                            FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "bmp"]).pick_file()
                        {
                            let path = path_buf.to_string_lossy().to_string();
                            let mut guard = state.lock().unwrap();
                            guard.custom_image_path = Some(Arc::new(path.clone()));
                            guard.add_toast(ToastType::Info, format!("Loaded image: {}", path), 2.0, true, None);
                        }
                    }
                }

                let mut load_btn = Declaration::<Texture2D, ()>::new();
                load_btn
                    .id(load_id)
                    .layout()
                    .width(grow!())
                    .padding(Padding::all(10))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end()
                    .background_color(load_color)
                    .corner_radius()
                    .all(8.0 * font_scale)
                    .end();

                let (path_label, path_color) = {
                    let g = state.lock().unwrap();
                    match &g.custom_image_path {
                        Some(p) => (p.split('/').last().unwrap_or("Image").to_string(), theme.cl_text_main),
                        None => ("No image loaded".to_string(), theme.cl_text_sub),
                    }
                };

                clay_scope.with(&load_btn, |clay| {
                    clay.text(
                        arena.push(format!("{}   {}", ICON_IMAGE, path_label)),
                        clay_layout::text::TextConfig::new()
                            .font_size((14.0 * font_scale) as u16)
                            .color(path_color)
                            .end(),
                    );
                });

                if is_processing {
                    let mut spinner = Declaration::<Texture2D, ()>::new();
                    spinner
                        .layout()
                        .width(fixed!(24.0 * font_scale))
                        .height(fixed!(24.0 * font_scale))
                        .end();
                    clay_scope.with(&spinner, |clay| {
                        clay.text(
                            ICON_SPINNER,
                            clay_layout::text::TextConfig::new()
                                .font_size((20.0 * font_scale) as u16)
                                .color(theme.cl_primary)
                                .end(),
                        );
                    });
                }
            });

            let custom_path = state.lock().unwrap().custom_image_path.clone();
            if let Some(p) = custom_path {
                // Fidelity Sliders
                let (low_fid, high_fid) = {
                    let g = state.lock().unwrap();
                    (g.img_low_fidelity, g.img_high_fidelity)
                };
                render_slider(clay_scope, "img_low_fid", "Low Fidelity (White)", low_fid, 0.0, 1.0, COLOR_SLIDER_X, state, |s, v| s.img_low_fidelity = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                render_slider(clay_scope, "img_high_fid", "High Fidelity (Black)", high_fid, 0.0, 1.0, COLOR_SLIDER_Y, state, |s, v| s.img_high_fidelity = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);

                let mut action_row = Declaration::<Texture2D, ()>::new();
                action_row
                    .layout()
                    .width(grow!())
                    .direction(LayoutDirection::LeftToRight)
                    .child_gap(12)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end();

                clay_scope.with(&action_row, |clay_scope| {
                    let preview_id = clay_scope.id("preview_custom_image");
                    let is_active_preview = {
                        let g = state.lock().unwrap();
                        g.preview_pattern.as_ref().map(|p| **p == "custom_image").unwrap_or(false)
                    };
                    let mut preview_color = if is_active_preview { theme.cl_primary } else { theme.cl_bg_dark };
                    if !is_processing && clay_scope.pointer_over(preview_id) {
                        preview_color = theme.cl_primary_hover;
                        if mouse_pressed {
                            let mut g = state.lock().unwrap();
                            if is_active_preview {
                                g.preview_pattern = None;
                                g.preview_paths.clear();
                                g.preview_version += 1;
                            } else {
                                g.preview_pattern = Some(Arc::new("custom_image".to_string()));
                                g.preview_paths.clear();
                                g.preview_version += 1;
                                g.is_processing = true;
                                let config = g.get_image_burn_config();
                                let state_clone = Arc::clone(state);
                                let path_clone = (*p).clone();
                                std::thread::spawn(move || {
                                    if let Ok((gcode, _)) = generate_image_gcode(&path_clone, &config, None, true) {
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
                        "burn_custom_image",
                        "BURN",
                        state,
                        0.0,
                        0.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle || is_processing,
                        theme,
                    ) {
                        let config = {
                            let mut g = state.lock().unwrap();
                            g.is_burning = true;
                            g.burn_log_active = true;
                            g.is_processing = true;
                            g.get_image_burn_config()
                        };
                        let state_clone = Arc::clone(state);
                        let path_clone = p.clone();
                        std::thread::spawn(move || {
                            let result = std::panic::catch_unwind(|| {
                                generate_image_gcode(&path_clone, &config, None, false)
                            });
                            match result {
                                Ok(Ok((gcode, _))) => {
                                    let mut g = state_clone.lock().unwrap();
                                    g.send_command(gcode);
                                    g.is_processing = false;
                                }
                                _ => {
                                    state_clone.lock().unwrap().is_processing = false;
                                }
                            }
                        });
                    }
                    let path_clone = p.clone();
                    render_outline_btn(
                        clay_scope,
                        "outline_custom_image",
                        state,
                        || {
                            get_image_outline_gcode(&path_clone, &state.lock().unwrap().get_image_burn_config())
                        },
                        mouse_pressed,
                        font_scale,
                        !is_idle || is_processing,
                        theme,
                    );
                });
            }
        });

        // 2. Settings Section
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
                "IMAGE SETTINGS",
                clay_layout::text::TextConfig::new()
                    .font_size((12.0 * font_scale) as u16)
                    .color(theme.cl_text_sub)
                    .end(),
            );

            let (pwr, spd, scl, lines, bx, by, bw, bh, b_en) = {
                let g = state.lock().unwrap();
                (
                    g.power,
                    g.feed_rate,
                    g.scale,
                    g.img_lines_per_mm,
                    g.bounds.x,
                    g.bounds.y,
                    g.bounds.w,
                    g.bounds.h,
                    g.bounds.enabled,
                )
            };

            let mut row1 = Declaration::<Texture2D, ()>::new();
            row1.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
            clay_scope.with(&row1, |clay_scope| {
                render_slider(clay_scope, "i_pwr", "Power", pwr, 0.0, 1000.0, COLOR_SLIDER_POWER, state, |s, v| s.power = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                render_slider(clay_scope, "i_spd", "Speed", spd, 10.0, 6000.0, COLOR_SLIDER_SPEED, state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
            });

            let mut row2 = Declaration::<Texture2D, ()>::new();
            row2.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
            clay_scope.with(&row2, |clay_scope| {
                render_slider(clay_scope, "i_scl", "Scale", scl, 0.1, 10.0, COLOR_SLIDER_STEP, state, |s, v| s.scale = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                render_slider(clay_scope, "i_lpm", "Lines/mm", lines, 1.0, 20.0, COLOR_SLIDER_PASSES, state, |s, v| s.img_lines_per_mm = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
            });

            render_checkbox(clay_scope, "i_ben", "Enable Bounds", b_en, state, |s, v| s.bounds.enabled = v, mouse_pressed, font_scale, theme);

            if b_en {
                let mut grid = Declaration::<Texture2D, ()>::new();
                grid.layout().width(grow!()).direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&grid, |clay_scope| {
                    let mut r1 = Declaration::<Texture2D, ()>::new();
                    r1.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
                    clay_scope.with(&r1, |clay_scope| {
                        render_slider(clay_scope, "i_bx", "X", bx, 0.0, 400.0, COLOR_SLIDER_X, state, |s, v| s.bounds.x = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                        render_slider(clay_scope, "i_by", "Y", by, 0.0, 400.0, COLOR_SLIDER_Y, state, |s, v| s.bounds.y = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    });
                    let mut r2 = Declaration::<Texture2D, ()>::new();
                    r2.layout().direction(LayoutDirection::LeftToRight).child_gap(8).end();
                    clay_scope.with(&r2, |clay_scope| {
                        render_slider(clay_scope, "i_bw", "W", bw, 1.0, 400.0, COLOR_SLIDER_W, state, |s, v| s.bounds.w = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                        render_slider(clay_scope, "i_bh", "H", bh, 1.0, 400.0, COLOR_SLIDER_H, state, |s, v| s.bounds.h = v, mouse_pos, mouse_down, scroll_y, arena, font_scale, theme);
                    });
                });
            }
        });
    });
}
