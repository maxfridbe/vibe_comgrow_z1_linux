use crate::cli_and_helpers::generate_text_gcode;
use crate::icons::*;
use crate::state::{AppState, StringArena};
use crate::styles::*;
use crate::ui::{Section, render_burn_btn, render_checkbox, render_outline_btn, render_slider};
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
) where
    'a: 'render,
{
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().width(grow!()).height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();

    let is_idle = { state.lock().unwrap().machine_state == "Idle" };
    let (enabled, bx, by, bw, bh) = {
        let g = state.lock().unwrap();
        (g.bounds.enabled, g.bounds.x, g.bounds.y, g.bounds.w, g.bounds.h)
    };

    clay.with(&left_col, |clay_scope| {
        // 1. Boundary Settings
        let mut bounds_box = Declaration::<Texture2D, ()>::new();
        bounds_box
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

        clay_scope.with(&bounds_box, |clay_scope| {
            clay_scope.text(
                "BOUNDARY SETTINGS",
                clay_layout::text::TextConfig::new()
                    .font_size((14.0 * font_scale) as u16)
                    .color(COLOR_TEXT_MUTED)
                    .end(),
            );

            render_checkbox(
                clay_scope,
                "txt_bounds_enabled",
                "Enable Boundary Clipping",
                enabled,
                state,
                |s, v| s.bounds.enabled = v,
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
                        "txt_bound_x",
                        "X Pos",
                        bx,
                        0.0,
                        400.0,
                        COLOR_SLIDER_X,
                        state,
                        |s, v| s.bounds.x = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                    render_slider(
                        clay_scope,
                        "txt_bound_w",
                        "Width",
                        bw,
                        1.0,
                        400.0,
                        COLOR_SLIDER_W,
                        state,
                        |s, v| s.bounds.w = v,
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
                        "txt_bound_y",
                        "Y Pos",
                        by,
                        0.0,
                        400.0,
                        COLOR_SLIDER_Y,
                        state,
                        |s, v| s.bounds.y = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                    render_slider(
                        clay_scope,
                        "txt_bound_h",
                        "Height",
                        bh,
                        1.0,
                        400.0,
                        COLOR_SLIDER_H,
                        state,
                        |s, v| s.bounds.h = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                });
            });
        });

        // 2. Text Options
        let mut text_box = Declaration::<Texture2D, ()>::new();
        text_box
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

        clay_scope.with(&text_box, |clay_scope| {
            clay_scope.text(
                "TEXT OPTIONS",
                clay_layout::text::TextConfig::new()
                    .font_size((14.0 * font_scale) as u16)
                    .color(COLOR_TEXT_MUTED)
                    .end(),
            );

            let (
                content,
                font_name,
                is_bold,
                is_outline,
                l_space,
                line_space,
                curve_steps,
                lines_per_mm,
                available_fonts,
                dropdown_open,
                is_processing,
            ) = {
                let g = state.lock().unwrap();
                (
                    g.text_content.clone(),
                    g.text_font.clone(),
                    g.text_is_bold,
                    g.text_is_outline,
                    g.text_letter_spacing,
                    g.text_line_spacing,
                    g.text_curve_steps,
                    g.text_lines_per_mm,
                    g.available_fonts.clone(),
                    g.text_font_dropdown_open,
                    g.is_processing,
                )
            };

            // Font selection dropdown
            let mut font_row = Declaration::<Texture2D, ()>::new();
            font_row
                .layout()
                .width(grow!())
                .direction(LayoutDirection::LeftToRight)
                .child_gap(8)
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                .end();
            clay_scope.with(&font_row, |clay_scope| {
                clay_scope.text(
                    "Font:",
                    clay_layout::text::TextConfig::new()
                        .font_size((12.0 * font_scale) as u16)
                        .color(COLOR_TEXT_LABEL)
                        .end(),
                );

                let dropdown_id = clay_scope.id("font_dropdown");
                let mut dropdown_color = if dropdown_open {
                    COLOR_PRIMARY
                } else {
                    COLOR_BG_DARK
                };
                if clay_scope.pointer_over(dropdown_id) {
                    dropdown_color = COLOR_PRIMARY_HOVER;
                    if mouse_pressed {
                        let mut g = state.lock().unwrap();
                        g.text_font_dropdown_open = !g.text_font_dropdown_open;
                    }
                }

                let mut dropdown_box = Declaration::<Texture2D, ()>::new();
                dropdown_box
                    .id(dropdown_id)
                    .layout()
                    .width(grow!())
                    .padding(Padding::all(8))
                    .direction(LayoutDirection::LeftToRight)
                    .child_gap(8)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                    .end()
                    .background_color(dropdown_color)
                    .corner_radius()
                    .all(4.0 * font_scale)
                    .end();

                clay_scope.with(&dropdown_box, |clay_scope| {
                    clay_scope.text(
                        arena.push(font_name.clone()),
                        clay_layout::text::TextConfig::new()
                            .font_size((14.0 * font_scale) as u16)
                            .color(COLOR_TEXT_WHITE)
                            .end(),
                    );
                    let mut spacer = Declaration::<Texture2D, ()>::new();
                    spacer.layout().width(grow!()).end();
                    clay_scope.with(&spacer, |_| {});
                    clay_scope.text(
                        if dropdown_open {
                            ICON_ARROW_UP
                        } else {
                            ICON_ARROW_DOWN
                        },
                        clay_layout::text::TextConfig::new()
                            .font_size((12.0 * font_scale) as u16)
                            .color(COLOR_TEXT_WHITE)
                            .end(),
                    );
                });
            });

            if dropdown_open {
                let (fonts_count, scroll_offset) = {
                    let g = state.lock().unwrap();
                    (g.available_fonts.len(), g.text_font_scroll_offset)
                };

                let mut dropdown_list = Declaration::<Texture2D, ()>::new();
                let dropdown_list_id = clay_scope.id("font_dropdown_list");
                dropdown_list
                    .id(dropdown_list_id)
                    .layout()
                    .width(grow!())
                    .height(fixed!(200.0 * font_scale))
                    .direction(LayoutDirection::TopToBottom)
                    .end()
                    .background_color(COLOR_BG_DARK)
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
                    let max_scroll = -((fonts_count as f32 * 32.0 * font_scale) - (200.0 * font_scale)).max(0.0);
                    if g.text_font_scroll_offset < max_scroll {
                        g.text_font_scroll_offset = max_scroll;
                    }
                }

                clay_scope.with(&dropdown_list, |clay_scope| {
                    for font in available_fonts.iter() {
                        let item_id = clay_scope.id(arena.push(format!("font_item_{}", font)));
                        let mut item_color = COLOR_BG_DARK;
                        if clay_scope.pointer_over(item_id) {
                            item_color = COLOR_BG_SECTION;
                            if mouse_pressed {
                                let mut g = state.lock().unwrap();
                                g.text_font = font.clone();
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
                                    .color(COLOR_TEXT_WHITE)
                                    .end(),
                            );
                        });
                    }
                });
            }

            // Text content
            let mut input_row = Declaration::<Texture2D, ()>::new();
            input_row
                .layout()
                .width(grow!())
                .direction(LayoutDirection::LeftToRight)
                .child_gap(8)
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                .end();
            clay_scope.with(&input_row, |clay_scope| {
                clay_scope.text(
                    "Content:",
                    clay_layout::text::TextConfig::new()
                        .font_size((12.0 * font_scale) as u16)
                        .color(COLOR_TEXT_LABEL)
                        .end(),
                );

                let input_id = clay_scope.id("text_input");
                let is_active = { state.lock().unwrap().is_text_input_active };
                let mut input_color = if is_active {
                    COLOR_BG_SECTION
                } else {
                    COLOR_BG_DARK
                };
                let border_color = if is_active {
                    COLOR_PRIMARY
                } else {
                    COLOR_BG_DARK
                };

                if clay_scope.pointer_over(input_id) {
                    if !is_active {
                        input_color = COLOR_PRIMARY_HOVER;
                    }
                    if mouse_pressed {
                        let mut g = state.lock().unwrap();
                        g.is_text_input_active = true;
                        g.text_font_dropdown_open = false; // Close font list if typing
                    }
                } else if mouse_pressed {
                    let mut g = state.lock().unwrap();
                    g.is_text_input_active = false;
                }

                let mut box_decl = Declaration::<Texture2D, ()>::new();
                box_decl
                    .id(input_id)
                    .layout()
                    .width(grow!())
                    .padding(Padding::all(8))
                    .end()
                    .background_color(input_color)
                    .corner_radius()
                    .all(4.0 * font_scale)
                    .end()
                    .border()
                    .top((1.0 * font_scale) as u16)
                    .bottom((1.0 * font_scale) as u16)
                    .left((1.0 * font_scale) as u16)
                    .right((1.0 * font_scale) as u16)
                    .color(border_color)
                    .end();

                clay_scope.with(&box_decl, |clay_scope| {
                    let display_text = if is_active {
                        arena.push(format!(
                            "{}{}",
                            content,
                            if (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
                                / 500)
                                % 2
                                == 0
                            {
                                "|"
                            } else {
                                " "
                            }
                        ))
                    } else {
                        arena.push(content.clone())
                    };
                    clay_scope.text(
                        display_text,
                        clay_layout::text::TextConfig::new()
                            .font_size((14.0 * font_scale) as u16)
                            .color(COLOR_TEXT_WHITE)
                            .end(),
                    );
                });
            });

            let mut style_row = Declaration::<Texture2D, ()>::new();
            style_row.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&style_row, |clay_scope| {
                render_checkbox(
                    clay_scope,
                    "txt_bold",
                    "Bold",
                    is_bold,
                    state,
                    |s, v| s.text_is_bold = v,
                    mouse_pressed,
                    font_scale,
                );
                render_checkbox(
                    clay_scope,
                    "txt_outline",
                    "Outline",
                    is_outline,
                    state,
                    |s, v| s.text_is_outline = v,
                    mouse_pressed,
                    font_scale,
                );
            });

            let mut spacing_grid = Declaration::<Texture2D, ()>::new();
            spacing_grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&spacing_grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new();
                col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(
                        clay_scope,
                        "txt_lspace",
                        "Letter Spacing",
                        l_space,
                        -5.0,
                        20.0,
                        COLOR_SLIDER_X,
                        state,
                        |s, v| s.text_letter_spacing = v,
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
                        "txt_linespace",
                        "Line Spacing",
                        line_space,
                        0.5,
                        3.0,
                        COLOR_SLIDER_Y,
                        state,
                        |s, v| s.text_line_spacing = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                });
            });

            let mut quality_grid = Declaration::<Texture2D, ()>::new();
            quality_grid.layout().width(grow!()).direction(LayoutDirection::LeftToRight).child_gap(16).end();
            clay_scope.with(&quality_grid, |clay_scope| {
                let mut col1 = Declaration::<Texture2D, ()>::new();
                col1.layout().direction(LayoutDirection::TopToBottom).child_gap(8).end();
                clay_scope.with(&col1, |clay_scope| {
                    render_slider(
                        clay_scope,
                        "txt_curve_steps",
                        "Curve Smoothness",
                        curve_steps as f32,
                        1.0,
                        50.0,
                        COLOR_SLIDER_X,
                        state,
                        |s, v| s.text_curve_steps = v as u32,
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
                        "txt_lines_per_mm",
                        "Lines Per MM",
                        lines_per_mm,
                        1.0,
                        40.0,
                        COLOR_SLIDER_Y,
                        state,
                        |s, v| s.text_lines_per_mm = v,
                        mouse_pos,
                        mouse_down,
                        scroll_y,
                        arena,
                        font_scale,
                    );
                });
            });

            let mut action_row = Declaration::<Texture2D, ()>::new();
            action_row
                .layout()
                .child_gap(12)
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                .end();
            clay_scope.with(&action_row, |clay_scope| {
                // Preview Eyeball
                let eye_id = clay_scope.id("eye_text");
                let is_previewing = { state.lock().unwrap().preview_pattern.as_deref() == Some("text") };
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
                            g.preview_pattern = Some("text".to_string());
                            g.preview_paths.clear();
                            g.is_processing = true;
                            let config = g.get_text_burn_config();
                            let state_clone = Arc::clone(state);
                            std::thread::spawn(move || {
                                if let Ok((gcode, _)) = generate_text_gcode(
                                    &config,
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
                ) {
                    let state_data = {
                        let mut g = state.lock().unwrap();
                        g.is_processing = true;
                        g.get_text_burn_config()
                    };
                    let state_clone = Arc::clone(state);
                    std::thread::spawn(move || {
                        let config = state_data;
                        if let Ok((gcode, _)) = generate_text_gcode(
                            &config, false,
                        ) {
                            state_clone.lock().unwrap().send_command(gcode);
                        }
                        state_clone.lock().unwrap().is_processing = false;
                    });
                }
                render_outline_btn(
                    clay_scope,
                    "outline_text",
                    state,
                    || {
                        let g = state.lock().unwrap();
                        let config = g.get_text_burn_config();
                        generate_text_gcode(
                            &config,
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
                        "txt_power",
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
                        "txt_scale",
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
                        "txt_speed",
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
                        "txt_passes",
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
