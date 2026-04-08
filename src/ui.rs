use crate::cli_and_helpers;
use crate::icons::*;
use crate::state::{AppState, StringArena};
use crate::styles::*;
use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::{Color as ClayColor, Declaration, fixed, grow};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};

pub fn render_tab_btn<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    active: bool,
    font_scale: f32,
) -> bool
where
    'a: 'render,
{
    let icon = match label {
        "Manual" => ICON_MOVE,
        "Pattern" => ICON_GAUGE,
        "Image" => ICON_IMAGE,
        _ => ICON_TERMINAL,
    };

    let btn_id = clay.id(id);
    let mut color = if active {
        COLOR_PRIMARY
    } else {
        COLOR_BG_SECTION
    };
    let mut text_color = if active {
        COLOR_TEXT_WHITE
    } else {
        COLOR_TEXT_MUTED
    };

    if !active && clay.pointer_over(btn_id) {
        color = COLOR_PRIMARY_HOVER;
        text_color = COLOR_TEXT_WHITE;
    }

    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id)
        .layout()
        .padding(Padding::new(16, 16, 10, 10))
        .direction(LayoutDirection::LeftToRight)
        .child_gap(8)
        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
        .end()
        .background_color(color)
        .corner_radius()
        .top_left(8.0 * font_scale)
        .top_right(8.0 * font_scale)
        .end();

    let mut clicked = false;
    clay.with(&btn, |clay_scope| {
        clay_scope.text(
            icon,
            clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(text_color).end(),
        );
        clay_scope.text(
            label,
            clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(text_color).end(),
        );
        if unsafe { raylib::ffi::IsMouseButtonPressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT as i32) }
            && clay_scope.pointer_over(btn_id)
        {
            clicked = true;
        }
    });
    clicked
}

pub struct Command {
    pub label: &'static str,
    pub cmd: &'static str,
}

pub struct Section {
    pub title: &'static str,
    pub icon: &'static str,
    pub color: ClayColor,
    pub commands: Vec<Command>,
}

pub fn render_jog_btn<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    icon: &str,
    state: &Arc<Mutex<AppState>>,
    axis: &str,
    direction: f32,
    mouse_pressed: bool,
    clipboard: &mut Option<Clipboard>,
    font_scale: f32,
    disabled: bool,
) -> bool
where
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut color = if disabled {
        COLOR_BG_DISABLED
    } else {
        COLOR_BG_SECTION
    };
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = COLOR_PRIMARY;
        if mouse_pressed {
            clicked = true;
            let mut guard = state.lock().unwrap();
            let d = guard.distance;
            let cmd = crate::gcode::jog_axis(axis, direction * d, guard.feed_rate);
            guard.send_command(cmd.clone());
            if let Some(cb) = clipboard {
                let _ = cb.set_text(cmd);
            }
        }
    }
    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id)
        .layout()
        .width(fixed!(30.0 * font_scale))
        .height(fixed!(30.0 * font_scale))
        .padding(Padding::all(4))
        .direction(LayoutDirection::TopToBottom)
        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
        .end()
        .background_color(color)
        .corner_radius()
        .all(8.0 * font_scale)
        .end();

    let text_color = if disabled {
        COLOR_TEXT_DISABLED
    } else {
        COLOR_TEXT_WHITE
    };
    clay.with(&btn, |clay| {
        clay.text(
            icon,
            clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(text_color).end(),
        );
    });
    clicked
}

pub fn render_burn_btn<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    state: &Arc<Mutex<AppState>>,
    dx: f32,
    dy: f32,
    mouse_pressed: bool,
    clipboard: &mut Option<Clipboard>,
    font_scale: f32,
    disabled: bool,
) -> bool
where
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut color = if disabled {
        COLOR_BG_DISABLED
    } else {
        COLOR_ACCENT_PURPLE
    };
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = COLOR_ACCENT_PURPLE_LIGHT;
        if mouse_pressed {
            clicked = true;
            let mut guard = state.lock().unwrap();
            let d = guard.distance;
            let f = guard.feed_rate;
            let s = guard.power;
            let dx_scaled = dx * d;
            let dy_scaled = dy * d;
            let cmd = format!("{}\n{}", crate::gcode::CMD_RELATIVE_POS, crate::gcode::burn(dx_scaled, dy_scaled, s, f));
            guard.send_command(cmd.clone());
            if let Some(cb) = clipboard {
                let _ = cb.set_text(cmd);
            }
        }
    }
    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id)
        .layout()
        .width(fixed!(65.0 * font_scale))
        .padding(Padding::all(4))
        .direction(LayoutDirection::LeftToRight)
        .child_gap(4)
        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
        .end()
        .background_color(color)
        .corner_radius()
        .all(8.0 * font_scale)
        .end();

    let text_color = if disabled {
        COLOR_TEXT_DISABLED
    } else {
        COLOR_TEXT_WHITE
    };
    clay.with(&btn, |clay| {
        clay.text(
            ICON_FLAME,
            clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(text_color).end(),
        );
        clay.text(
            label,
            clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(text_color).end(),
        );
    });
    clicked
}

pub fn render_outline_btn<'a, 'render, F>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    state: &Arc<Mutex<AppState>>,
    action: F,
    mouse_pressed: bool,
    font_scale: f32,
    disabled: bool,
) -> bool
where
    F: FnOnce() -> Option<String>,
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut color = if disabled {
        COLOR_BG_DISABLED
    } else {
        COLOR_BG_DARK
    };
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = COLOR_PRIMARY_HOVER;
        if mouse_pressed {
            clicked = true;
            if let Some(gcode) = action() {
                if let Some((x, y, w, h)) = cli_and_helpers::get_gcode_bounds(&gcode) {
                    let mut guard = state.lock().unwrap();
                    let speed = guard.feed_rate;
                    let outline_gcode = cli_and_helpers::generate_outline_gcode(x, y, w, h, speed);
                    guard.send_command(outline_gcode);
                }
            }
        }
    }
    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id)
        .layout()
        .width(fixed!(35.0 * font_scale))
        .padding(Padding::all(4))
        .direction(LayoutDirection::TopToBottom)
        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
        .end()
        .background_color(color)
        .corner_radius()
        .all(8.0 * font_scale)
        .end();

    let text_color = if disabled {
        COLOR_TEXT_DISABLED
    } else {
        COLOR_TEXT_WHITE
    };
    clay.with(&btn, |clay| {
        clay.text(
            "[]",
            clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(text_color).end(),
        );
    });
    clicked
}

pub fn render_checkbox<'a, 'render, F>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    checked: bool,
    state: &Arc<Mutex<AppState>>,
    update: F,
    mouse_pressed: bool,
    font_scale: f32,
) where
    F: FnOnce(&mut AppState, bool),
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id)
        .layout()
        .padding(Padding::all(8))
        .child_gap(12)
        .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
        .end()
        .background_color(COLOR_BG_SECTION)
        .corner_radius()
        .all(8.0 * font_scale)
        .end();

    if clay.pointer_over(btn_id) && mouse_pressed {
        let mut guard = state.lock().unwrap();
        update(&mut guard, !checked);
    }

    clay.with(&btn, |clay_scope| {
        let mut box_decl = Declaration::<Texture2D, ()>::new();
        let box_color = if checked {
            COLOR_PRIMARY
        } else {
            COLOR_BG_DARK
        };
        box_decl
            .layout()
            .width(fixed!(20.0 * font_scale))
            .height(fixed!(20.0 * font_scale))
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
            .end()
            .background_color(box_color)
            .corner_radius()
            .all(4.0 * font_scale)
            .end();

        clay_scope.with(&box_decl, |clay_scope| {
            if checked {
                clay_scope.text(
                    ICON_CHECK,
                    clay_layout::text::TextConfig::new()
                        .font_size((14.0 * font_scale) as u16)
                        .color(COLOR_TEXT_WHITE)
                        .end(),
                );
            }
        });

        clay_scope.text(
            label,
            clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(COLOR_TEXT_MUTED).end(),
        );
    });
}

pub fn render_slider<'a, 'render, F>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    color: ClayColor,
    state: &Arc<Mutex<AppState>>,
    update: F,
    mouse_pos: raylib::math::Vector2,
    mouse_down: bool,
    _scroll_y: f32,
    arena: &StringArena,
    font_scale: f32,
) where
    F: FnOnce(&mut AppState, f32),
    'a: 'render,
{
    let slider_id = clay.id(id);
    let container_id = clay.id(arena.push(format!("{}_container", id)));
    let btn_minus_id = clay.id(arena.push(format!("{}_minus", id)));
    let btn_plus_id = clay.id(arena.push(format!("{}_plus", id)));

    let mut container = Declaration::<Texture2D, ()>::new();
    container
        .id(container_id)
        .layout()
        .width(fixed!(180.0 * font_scale))
        .direction(LayoutDirection::TopToBottom)
        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
        .child_gap(4)
        .end();

    let mut next_val = None;
    let raw_step = (max - min) * 0.05;
    let step = if max - min > 10.0 {
        raw_step.round().max(1.0)
    } else {
        (raw_step * 10.0).round().max(0.1) / 10.0
    };

    clay.with(&container, |clay| {
        let mut header = Declaration::<Texture2D, ()>::new();
        header
            .layout()
            .width(grow!())
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
            .end();
        clay.with(&header, |clay| {
            clay.text(
                label,
                clay_layout::text::TextConfig::new()
                    .font_size((14.0 * font_scale) as u16)
                    .color(COLOR_TEXT_LABEL)
                    .end(),
            );

            // Intelligent rounding
            let val_str = if value.fract() == 0.0 {
                format!("{}", value as i32)
            } else {
                format!("{:.1}", value)
            };
            clay.text(
                arena.push(val_str),
                clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(color).end(),
            );
        });

        let mut slider_row = Declaration::<Texture2D, ()>::new();
        slider_row
            .layout()
            .width(grow!())
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
            .child_gap(8)
            .end();

        clay.with(&slider_row, |clay| {
            // Minus Button
            let mut minus_box = Declaration::<Texture2D, ()>::new();
            let mut minus_bg = COLOR_BG_DARK;
            if clay.pointer_over(btn_minus_id) {
                minus_bg = COLOR_PRIMARY_HOVER;
                if unsafe { raylib::ffi::IsMouseButtonPressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT as i32) } {
                    let mut nv = value - step;
                    if max - min > 10.0 {
                        nv = nv.round();
                    } else {
                        nv = (nv * 10.0).round() / 10.0;
                    }
                    next_val = Some(nv.clamp(min, max));
                }
            }
            minus_box
                .id(btn_minus_id)
                .layout()
                .width(fixed!(16.0 * font_scale))
                .height(fixed!(16.0 * font_scale))
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end()
                .background_color(minus_bg)
                .corner_radius()
                .all(4.0 * font_scale)
                .end();
            clay.with(&minus_box, |clay| {
                clay.text(
                    "-",
                    clay_layout::text::TextConfig::new()
                        .font_size((14.0 * font_scale) as u16)
                        .color(COLOR_TEXT_WHITE)
                        .end(),
                );
            });

            // Track
            let mut track = Declaration::<Texture2D, ()>::new();
            track
                .id(slider_id)
                .layout()
                .width(grow!())
                .height(fixed!(6.0 * font_scale))
                .end()
                .background_color(COLOR_BG_DARK)
                .corner_radius()
                .all(3.0 * font_scale)
                .end();

            if clay.pointer_over(slider_id) && mouse_down {
                let data = unsafe { clay_layout::bindings::Clay_GetElementData(slider_id.id) };
                if data.found {
                    let rect = data.boundingBox;
                    let mouse_x = mouse_pos.x;
                    let percent = ((mouse_x - rect.x) / rect.width).clamp(0.0, 1.0);
                    let raw_val = min + percent * (max - min);
                    // Intelligent rounding: round to integer if range is large, else 1 decimal
                    if max - min > 10.0 {
                        next_val = Some(raw_val.round());
                    } else {
                        next_val = Some((raw_val * 10.0).round() / 10.0);
                    }
                }
            }

            clay.with(&track, |clay| {
                let mut bar = Declaration::<Texture2D, ()>::new();
                let percent = (value - min) / (max - min);
                bar.layout()
                    .width(fixed!(percent * 130.0 * font_scale))
                    .height(grow!())
                    .end()
                    .background_color(color)
                    .corner_radius()
                    .all(3.0 * font_scale)
                    .end();
                clay.with(&bar, |_| {});
            });

            // Plus Button
            let mut plus_box = Declaration::<Texture2D, ()>::new();
            let mut plus_bg = COLOR_BG_DARK;
            if clay.pointer_over(btn_plus_id) {
                plus_bg = COLOR_PRIMARY_HOVER;
                if unsafe { raylib::ffi::IsMouseButtonPressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT as i32) } {
                    let mut nv = value + step;
                    if max - min > 10.0 {
                        nv = nv.round();
                    } else {
                        nv = (nv * 10.0).round() / 10.0;
                    }
                    next_val = Some(nv.clamp(min, max));
                }
            }
            plus_box
                .id(btn_plus_id)
                .layout()
                .width(fixed!(16.0 * font_scale))
                .height(fixed!(16.0 * font_scale))
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end()
                .background_color(plus_bg)
                .corner_radius()
                .all(4.0 * font_scale)
                .end();
            clay.with(&plus_box, |clay| {
                clay.text(
                    "+",
                    clay_layout::text::TextConfig::new()
                        .font_size((14.0 * font_scale) as u16)
                        .color(COLOR_TEXT_WHITE)
                        .end(),
                );
            });
        });
    });

    if let Some(nv) = next_val {
        let mut guard = state.lock().unwrap();
        update(&mut guard, nv);
    }
}
