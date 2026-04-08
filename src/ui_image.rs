use crate::cli_and_helpers::generate_image_gcode;
use crate::icons::*;
use crate::state::{AppState, StringArena};
use crate::styles::*;
use crate::ui::{Section, render_burn_btn, render_checkbox, render_outline_btn, render_slider};
use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::{Declaration, grow};
use raylib::prelude::*;
use rfd::FileDialog;
use std::path::Path;
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
) where
    'a: 'render,
{
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().width(grow!()).height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();

    let is_idle = { state.lock().unwrap().machine_state == "Idle" };
    let (enabled, bx, by, bw, bh) = {
        let g = state.lock().unwrap();
        (g.boundary_enabled, g.boundary_x, g.boundary_y, g.boundary_w, g.boundary_h)
    };

    clay.with(&left_col, |clay_scope| {
        // 1. Boundary Settings
        let mut boundary_box = Declaration::<Texture2D, ()>::new();
        boundary_box
            .layout()
            .width(grow!())
            .direction(LayoutDirection::TopToBottom)
            .padding(Padding::all(12))
            .child_gap(12)
            .end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();

        clay_scope.with(&boundary_box, |clay_scope| {
            clay_scope.text(
                "BOUNDARY SETTINGS",
                clay_layout::text::TextConfig::new()
                    .font_size((14.0 * font_scale) as u16)
                    .color(COLOR_TEXT_MUTED)
                    .end(),
            );

            render_checkbox(
                clay_scope,
                "img_boundary_enabled",
                "Enable Boundary Clipping",
                enabled,
                state,
                |s, v| s.boundary_enabled = v,
                mouse_pressed,
                font_scale,
            );

            let mut grid = Declaration::<Texture2D, ()>::new();
            grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new();
                col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(
                        clay_scope,
                        "img_bound_x",
                        "X Pos",
                        bx,
                        0.0,
                        400.0,
                        COLOR_SLIDER_X,
                        state,
                        |s, v| s.boundary_x = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                    render_slider(
                        clay_scope,
                        "img_bound_w",
                        "Width",
                        bw,
                        1.0,
                        400.0,
                        COLOR_SLIDER_W,
                        state,
                        |s, v| s.boundary_w = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                });
                let mut col2 = Declaration::<Texture2D, ()>::new();
                col2.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(
                        clay_scope,
                        "img_bound_y",
                        "Y Pos",
                        by,
                        0.0,
                        400.0,
                        COLOR_SLIDER_Y,
                        state,
                        |s, v| s.boundary_y = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                    render_slider(
                        clay_scope,
                        "img_bound_h",
                        "Height",
                        bh,
                        1.0,
                        400.0,
                        COLOR_SLIDER_H,
                        state,
                        |s, v| s.boundary_h = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                });
            });
        });

        // 2. Image Loading Pane
        let mut img_box = Declaration::<Texture2D, ()>::new();
        img_box
            .layout()
            .width(grow!())
            .direction(LayoutDirection::TopToBottom)
            .padding(Padding::all(12))
            .child_gap(12)
            .end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();

        clay_scope.with(&img_box, |clay_scope| {
            clay_scope.text(
                "IMAGE LOADING",
                clay_layout::text::TextConfig::new()
                    .font_size((14.0 * font_scale) as u16)
                    .color(COLOR_TEXT_MUTED)
                    .end(),
            );

            let load_id = clay_scope.id("pick_image_btn");
            let mut load_color = if !is_idle {
                COLOR_BG_DISABLED
            } else {
                COLOR_PRIMARY_HOVER
            };
            if is_idle && clay_scope.pointer_over(load_id) {
                load_color = COLOR_PRIMARY;
                if mouse_pressed {
                    if let Some(path_buf) =
                        FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "bmp"]).pick_file()
                    {
                        state.lock().unwrap().custom_image_path = Some(path_buf.to_string_lossy().to_string());
                    }
                }
            }

            let mut load_btn = Declaration::<Texture2D, ()>::new();
            load_btn
                .id(load_id)
                .layout()
                .padding(Padding::all(10))
                .direction(LayoutDirection::LeftToRight)
                .child_gap(8)
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end()
                .background_color(load_color)
                .corner_radius()
                .all(8.0 * font_scale)
                .end();

            let load_text_color = if !is_idle {
                COLOR_TEXT_DISABLED
            } else {
                COLOR_TEXT_WHITE
            };
            clay_scope.with(&load_btn, |clay| {
                clay.text(
                    arena.push(format!("{}   Pick Image", ICON_IMAGE)),
                    clay_layout::text::TextConfig::new()
                        .font_size((14.0 * font_scale) as u16)
                        .color(load_text_color)
                        .end(),
                );
            });


            // Fidelity Sliders immediately under Pick button
            let (low_fid, high_fid, is_processing) = {
                let g = state.lock().unwrap();
                (g.img_low_fidelity, g.img_high_fidelity, g.is_processing)
            };
            render_slider(
                clay_scope,
                "img_low_fid",
                "Low Fidelity (White)",
                low_fid,
                0.0,
                1.0,
                COLOR_SLIDER_X,
                state,
                |s, v| s.img_low_fidelity = v,
                mouse_pos,
                mouse_down,
                scroll_y,
                arena,
                font_scale,
            );
            render_slider(
                clay_scope,
                "img_high_fid",
                "High Fidelity (Black)",
                high_fid,
                0.0,
                1.0,
                COLOR_SLIDER_Y,
                state,
                |s, v| s.img_high_fidelity = v,
                mouse_pos,
                mouse_down,
                scroll_y,
                arena,
                font_scale,
            );

            let custom_path = { state.lock().unwrap().custom_image_path.clone() };
            if let Some(p) = custom_path {
                let filename = Path::new(&p).file_name().and_then(|f| f.to_str()).unwrap_or("unknown");

                let mut file_info_row = Declaration::<Texture2D, ()>::new();
                file_info_row
                    .layout()
                    .child_gap(12)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                    .end();

                clay_scope.with(&file_info_row, |clay_scope| {
                    // Preview Eyeball first
                    let eye_id = clay_scope.id("eye_custom_image");
                    let is_previewing = { state.lock().unwrap().preview_pattern.as_deref() == Some("custom_image") };
                    let mut eye_color = if is_previewing {
                        COLOR_SUCCESS
                    } else {
                        COLOR_TEXT_MUTED
                    };
                    if !is_processing && clay_scope.pointer_over(eye_id) {
                        eye_color = COLOR_TEXT_WHITE;
                        if mouse_pressed {
                            let mut g = state.lock().unwrap();
                            if is_previewing {
                                g.preview_pattern = None;
                                g.preview_paths.clear();
                            } else {
                                g.preview_pattern = Some("custom_image".to_string());
                                g.preview_paths.clear();
                                g.is_processing = true;
                                let config = g.get_image_burn_config();
                                let fit = if config.base.boundary_enabled {
                                    Some((config.base.boundary_w, config.base.boundary_h))
                                } else {
                                    None
                                };
                                let center = if config.base.boundary_enabled {
                                    (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                                } else {
                                    (200.0, 200.0)
                                };
                                let state_clone = Arc::clone(state);
                                let path_clone = p.clone();
                                std::thread::spawn(move || {
                                    if let Ok((gcode, _)) = generate_image_gcode(
                                        &path_clone,
                                        config.base.power,
                                        config.base.feed_rate * 10.0,
                                        config.base.scale,
                                        config.base.passes,
                                        fit,
                                        center,
                                        config.low_fid,
                                        config.high_fid,
                                        config.lines_per_mm,
                                        true,
                                    ) {
                                        let mut g = state_clone.lock().unwrap();
                                        let original_v_pos = g.v_pos;
                                        let original_is_abs = g.is_absolute;
                                        for line in gcode.lines() {
                                            g.process_command_for_preview(line);
                                        }
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
                        if is_processing {
                            clay.text(
                                ICON_SPINNER,
                                clay_layout::text::TextConfig::new()
                                    .font_size((20.0 * font_scale) as u16)
                                    .color(COLOR_SUCCESS)
                                    .end(),
                            );
                        } else {
                            clay.text(
                                ICON_EYE,
                                clay_layout::text::TextConfig::new()
                                    .font_size((20.0 * font_scale) as u16)
                                    .color(eye_color)
                                    .end(),
                            );
                        }
                    });

                    clay_scope.text(
                        arena.push(filename.to_string()),
                        clay_layout::text::TextConfig::new()
                            .font_size((14.0 * font_scale) as u16)
                            .color(COLOR_TEXT_WHITE)
                            .end(),
                    );

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
                        !is_idle,
                    ) {
                        let config = {
                            let mut g = state.lock().unwrap();
                            g.is_processing = true;
                            g.get_image_burn_config()
                        };
                        let fit = if config.base.boundary_enabled {
                            Some((config.base.boundary_w, config.base.boundary_h))
                        } else {
                            None
                        };
                        let center = if config.base.boundary_enabled {
                            (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                        } else {
                            (200.0, 200.0)
                        };
                        let state_clone = Arc::clone(state);
                        let path_clone = p.clone();
                        std::thread::spawn(move || {
                            if let Ok((gcode, _)) =
                                generate_image_gcode(&path_clone, config.base.power, config.base.feed_rate, config.base.scale, config.base.passes, fit, center, config.low_fid, config.high_fid, config.lines_per_mm, false)
                            {
                                state_clone.lock().unwrap().send_command(gcode);
                            }
                            state_clone.lock().unwrap().is_processing = false;
                        });
                    }
                    let path_clone = p.clone();
                    render_outline_btn(
                        clay_scope,
                        "outline_custom_image",
                        state,
                        move || {
                            let config = state.lock().unwrap().get_image_burn_config();
                            let fit = if config.base.boundary_enabled {
                                Some((config.base.boundary_w, config.base.boundary_h))
                            } else {
                                None
                            };
                            let center = if config.base.boundary_enabled {
                                (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                            } else {
                                (200.0, 200.0)
                            };
                            generate_image_gcode(
                                &path_clone,
                                config.base.power,
                                config.base.feed_rate,
                                config.base.scale,
                                config.base.passes,
                                fit,
                                center,
                                config.low_fid,
                                config.high_fid,
                                config.lines_per_mm,
                                false,
                            )
                            .ok()
                            .map(|(g, _)| g)
                        },
                        mouse_pressed,
                        font_scale,
                        !is_idle,
                    );
                });
            }
        });

        // 3. Sliders (TROGDOR)
        let mut controls_box = Declaration::<Texture2D, ()>::new();
        controls_box
            .layout()
            .width(grow!())
            .direction(LayoutDirection::TopToBottom)
            .padding(Padding::all(12))
            .child_gap(16)
            .end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();

        clay_scope.with(&controls_box, |clay_scope| {
            clay_scope.text(
                "TROGDOR",
                clay_layout::text::TextConfig::new()
                    .font_size((14.0 * font_scale) as u16)
                    .color(COLOR_TEXT_MUTED)
                    .end(),
            );

            let (pwr, spd, scl, pas) = {
                let g = state.lock().unwrap();
                (g.power, g.feed_rate, g.scale, g.passes)
            };

            let mut grid = Declaration::<Texture2D, ()>::new();
            grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new();
                col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(
                        clay_scope,
                        "img_power",
                        "Power",
                        pwr,
                        0.0,
                        1000.0,
                        COLOR_SLIDER_POWER,
                        state,
                        |s, v| s.power = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                    render_slider(
                        clay_scope,
                        "img_scale",
                        "Scale",
                        scl,
                        0.1,
                        5.0,
                        COLOR_SLIDER_STEP,
                        state,
                        |s, v| s.scale = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                });
                let mut col2 = Declaration::<Texture2D, ()>::new();
                col2.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col2, |clay_scope| {
                    render_slider(
                        clay_scope,
                        "img_speed",
                        "Speed",
                        spd,
                        10.0,
                        6000.0,
                        COLOR_SLIDER_SPEED,
                        state,
                        |s, v| s.feed_rate = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                    render_slider(
                        clay_scope,
                        "img_passes",
                        "Passes",
                        pas as f32,
                        1.0,
                        20.0,
                        COLOR_SLIDER_PASSES,
                        state,
                        |s, v| s.passes = v as u32,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                });
            });
        });
    });
}
