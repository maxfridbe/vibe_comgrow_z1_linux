use crate::cli_and_helpers;
use crate::icons::*;
use crate::state::{AppState, StringArena, ToastType};
use crate::styles::*;
use crate::theme::Theme;
use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::math::Vector2 as ClayVector2;
use clay_layout::{Color as ClayColor, Declaration, fixed, grow};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};

use crate::FontMeasureEx;

pub static mut MEASURE_FONT_PTR: *const raylib::prelude::Font = std::ptr::null();

pub fn render_log<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    scroll_delta: raylib::math::Vector2,
    arena: &StringArena,
    font_scale: f32,
    theme: &Theme,
) where
    'a: 'render,
{
    let mut log_box = Declaration::<Texture2D, ()>::new();
    let serial_id_node = clay.id("serial_box");
    log_box
        .id(serial_id_node)
        .layout()
        .width(grow!())
        .height(grow!())
        .padding(Padding::new((8.0 * font_scale) as u16, 12, 12, 12))
        .direction(LayoutDirection::TopToBottom)
        .child_gap(4)
        .end()
        .background_color(theme.cl_bg_dark)
        .border()
        .top((2.0 * font_scale) as u16)
        .color(theme.cl_accent)
        .end();

    clay.with(&log_box, |clay| {
        let mut title_decl = Declaration::<Texture2D, ()>::new();
        title_decl
            .layout()
            .width(grow!())
            .padding(Padding::all(8))
            .child_alignment(Alignment::new(LayoutAlignmentX::Right, LayoutAlignmentY::Top))
            .end()
            .floating()
            .attach_to(clay_layout::elements::FloatingAttachToElement::Parent)
            .end();
        clay.with(&title_decl, |clay| {
            clay.text(
                "SERIAL LOG",
                clay_layout::text::TextConfig::new()
                    .font_size((12.0 * font_scale) as u16)
                    .color(theme.cl_text_label)
                    .end(),
            );
        });

        let logs = state.lock().unwrap().serial_logs.clone();
        let offset = state.lock().unwrap().log_scroll_offset;
        let mut log_scroll = Declaration::<Texture2D, ()>::new();
        let log_scroll_id = clay.id("log_scroll");
        log_scroll
            .id(log_scroll_id)
            .layout()
            .width(grow!())
            .height(grow!())
            .direction(LayoutDirection::TopToBottom)
            .child_gap(2)
            .end()
            .clip(
                false,
                true,
                ClayVector2 {
                    x: 0.0,
                    y: offset,
                },
            );

        if clay.pointer_over(log_scroll_id) {
            let mut g = state.lock().unwrap();
            g.log_scroll_offset += scroll_delta.y * 40.0;
            if g.log_scroll_offset > 0.0 {
                g.log_scroll_offset = 0.0;
            }
            let max_scroll = -(logs.len() as f32 * 20.0);
            if g.log_scroll_offset < max_scroll {
                g.log_scroll_offset = max_scroll;
            }
        }

        clay.with(&log_scroll, |clay| {
            for (i, log) in logs.iter().rev().take(1000).enumerate() {
                let text_color = if log.is_response {
                    theme.cl_bg_dark
                } else if i == 0 {
                    theme.cl_text_main
                } else {
                    theme.cl_text_sub
                };
                let mut row = Declaration::<Texture2D, ()>::new();
                row.layout()
                    .width(grow!())
                    .padding(Padding::horizontal(8))
                    .padding(Padding::vertical(2))
                    .child_gap(10)
                    .end();
                if log.is_response {
                    row.background_color(theme.cl_text_main).corner_radius().all(4.0 * font_scale).end();
                }
                clay.with(&row, |clay| {
                    clay.text(
                        arena.push(format!("[{}] {} {}", log.timestamp, log.text, log.explanation)),
                        clay_layout::text::TextConfig::new()
                            .font_size((11.0 * font_scale) as u16)
                            .color(text_color)
                            .end(),
                    );
                });
            }
        });
    });
}

pub fn render_dropdown<'a, 'render, F, G, H>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    selected_label: &str,
    items: &[String],
    is_open: bool,
    scroll_offset: f32,
    state: &Arc<Mutex<AppState>>,
    arena: &StringArena,
    font_scale: f32,
    theme: &Theme,
    mouse_pressed: bool,
    scroll_y: f32,
    on_toggle: F,
    on_select: G,
    on_scroll: H,
) where
    F: Fn(&mut AppState),
    G: Fn(&mut AppState, String),
    H: Fn(&mut AppState, f32),
    'a: 'render,
{
    let dropdown_id = clay.id(id);
    let mut dropdown_color = if is_open {
        theme.cl_primary
    } else {
        theme.cl_bg_dark
    };

    if clay.pointer_over(dropdown_id) {
        dropdown_color = theme.cl_primary_hover;
        if mouse_pressed {
            let mut g = state.lock().unwrap();
            on_toggle(&mut g);
        }
    }

    let mut dropdown_btn = Declaration::<Texture2D, ()>::new();
    dropdown_btn
        .id(dropdown_id)
        .layout()
        .width(grow!())
        .height(fixed!(theme.sz_btn_height * font_scale))
        .padding(Padding::horizontal(10))
        .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
        .end()
        .background_color(dropdown_color)
        .corner_radius()
        .top_left(8.0 * font_scale)
        .top_right(8.0 * font_scale)
        .bottom_left(if is_open { 0.0 } else { 8.0 * font_scale })
        .bottom_right(if is_open { 0.0 } else { 8.0 * font_scale })
        .end();

    if is_open {
        dropdown_btn
            .border()
            .top((1.0 * font_scale) as u16)
            .left((1.0 * font_scale) as u16)
            .right((1.0 * font_scale) as u16)
            .color(theme.cl_primary)
            .end();
    }

    clay.with(&dropdown_btn, |clay| {
        clay.text(
            arena.push(format!("{}   {}", ICON_FONT, selected_label)),
            clay_layout::text::TextConfig::new()
                .font_size((14.0 * font_scale) as u16)
                .color(theme.cl_text_main)
                .end(),
        );
    });

    if is_open {
        let dropdown_list_id = clay.id(arena.push(format!("{}_list", id)));
        let mut dropdown_list = Declaration::<Texture2D, ()>::new();
        dropdown_list
            .id(dropdown_list_id)
            .layout()
            .width(grow!())
            .height(fixed!(200.0 * font_scale))
            .direction(LayoutDirection::TopToBottom)
            .end()
            .floating()
            .attach_to(clay_layout::elements::FloatingAttachToElement::Parent)
            .offset(ClayVector2 { x: 0.0, y: theme.sz_btn_height * font_scale })
            .z_index(1000)
            .end()
            .background_color(theme.cl_bg_dark)
            .border()
            .left((1.0 * font_scale) as u16)
            .right((1.0 * font_scale) as u16)
            .bottom((1.0 * font_scale) as u16)
            .color(theme.cl_primary)
            .end()
            .corner_radius()
            .bottom_left(8.0 * font_scale)
            .bottom_right(8.0 * font_scale)
            .end()
            .clip(
                false,
                true,
                ClayVector2 {
                    x: 0.0,
                    y: scroll_offset,
                },
            );

        if clay.pointer_over(dropdown_list_id) {
            let mut g = state.lock().unwrap();
            let mut new_offset = scroll_offset + scroll_y * 40.0;
            if new_offset > 0.0 {
                new_offset = 0.0;
            }
            let items_count = items.len();
            let max_scroll = -((items_count as f32 * 32.0 * font_scale) - (200.0 * font_scale)).max(0.0);
            if new_offset < max_scroll {
                new_offset = max_scroll;
            }
            on_scroll(&mut g, new_offset);
        }

        clay.with(&dropdown_list, |clay_scope| {
            for item in items.iter() {
                let item_id = clay_scope.id(arena.push(format!("{}_item_{}", id, item)));
                let mut item_color = theme.cl_bg_dark;
                if clay_scope.pointer_over(item_id) {
                    item_color = theme.cl_bg_section;
                    if mouse_pressed {
                        let mut g = state.lock().unwrap();
                        on_select(&mut g, item.clone());
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
                        arena.push(item.clone()),
                        clay_layout::text::TextConfig::new()
                            .font_size((12.0 * font_scale) as u16)
                            .color(theme.cl_text_main)
                            .end(),
                    );
                });
            }
        });
    }
}

pub fn render_text_input<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    _id: &str,
    state: &Arc<Mutex<AppState>>,
    arena: &StringArena,
    font_scale: f32,
    theme: &Theme,
    mouse_pressed: bool,
) where
    'a: 'render,
{
    let (is_active, content, cursor_idx) = {
        let g = state.lock().unwrap();
        (g.is_text_input_active, (*g.text_content).clone(), g.text_cursor_index)
    };

    let mut input_color = theme.cl_bg_dark;
    let border_color = if is_active {
        theme.cl_primary
    } else {
        theme.cl_bg_section
    };

    let input_id = unsafe { clay_layout::id::Id { id: clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from("text_input_global"), 0, 0) } };
    
    if clay.pointer_over(input_id) {
        if !is_active {
            input_color = theme.cl_bg_section;
        }
        if mouse_pressed {
            let mut g = state.lock().unwrap();
            if !g.is_text_input_active {
                g.is_text_input_active = true;
                g.text_cursor_index = (*g.text_content).len();
            }
            g.text_font_dropdown_open = false;
        }
    }

    let mut input_box_decl = Declaration::<Texture2D, ()>::new();
    input_box_decl
        .id(input_id)
        .layout()
        .width(grow!())
        .height(fixed!(100.0 * font_scale))
        .padding(Padding::all(12))
        .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Top))
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

    clay.with(&input_box_decl, |clay| {
        let is_empty = content.is_empty();
        let display_text = if is_empty && !is_active {
            "Type here..."
        } else {
            &content
        };
        let display_color = if is_empty && !is_active {
            theme.cl_text_sub
        } else {
            theme.cl_text_main
        };

        let font_size = 16.0 * font_scale;
        
        clay.text(
            arena.push(display_text.to_string()),
            clay_layout::text::TextConfig::new()
                .font_size(font_size as u16)
                .color(display_color)
                .end(),
        );

        // Render Caret (Floating relative to the input box)
        if is_active && (unsafe { raylib::ffi::GetTime() } * 2.0) as i32 % 2 == 0 {
            let mut caret_x = 0.0;
            let mut caret_y = 0.0;
            
            unsafe {
                if !MEASURE_FONT_PTR.is_null() {
                    let f = &*MEASURE_FONT_PTR;
                    let lines: Vec<&str> = content.split('\n').collect();
                    let mut current_byte_idx = 0;
                    
                    let cursor = cursor_idx.min(content.len());
                    
                    for (i, line) in lines.iter().enumerate() {
                        let line_len = line.len();
                        if cursor >= current_byte_idx && cursor <= current_byte_idx + line_len {
                            let prefix_len = cursor - current_byte_idx;
                            let prefix = &line[..prefix_len];
                            let m = f.measure_text_ex(prefix, font_size, 0.0);
                            caret_x = m.x;
                            caret_y = i as f32 * font_size;
                            break;
                        }
                        current_byte_idx += line_len + 1;
                    }
                }
            }

            let mut caret = Declaration::<Texture2D, ()>::new();
            caret.layout()
                .width(fixed!(2.0 * font_scale))
                .height(fixed!(font_size))
                .end()
                .floating()
                .attach_to(clay_layout::elements::FloatingAttachToElement::Parent)
                // Add 12.0 padding manually because we are floating relative to the box start
                .offset(ClayVector2 { x: 12.0 + caret_x, y: 12.0 + caret_y })
                .end()
                .background_color(theme.cl_primary);
            
            clay.with(&caret, |_| {});
        }
    });
}


pub fn render_toasts<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    arena: &StringArena,
    font_scale: f32,
    mouse_pressed: bool,
    theme: &Theme,
) where
    'a: 'render,
{
    let active_toasts = {
        let guard = state.lock().unwrap();
        guard.active_toasts.clone()
    };

    if active_toasts.is_empty() {
        return;
    }

    let mut toasts_container = Declaration::<Texture2D, ()>::new();
    toasts_container
        .layout()
        .width(grow!())
        .direction(LayoutDirection::TopToBottom)
        .child_gap(8)
        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
        .padding(Padding::all(16))
        .end()
        .floating()
        .attach_to(clay_layout::elements::FloatingAttachToElement::Root)
        .z_index(2000)
        .end();

    let mut dismiss_ids = Vec::new();
    let mut action_ids = Vec::new();

    clay.with(&toasts_container, |clay_scope| {
        for toast in active_toasts {
            let bg_color = match toast.toast_type {
                ToastType::Info => theme.cl_primary,
                ToastType::Warning => theme.cl_accent,
                ToastType::Error => theme.cl_danger,
            };

            let mut toast_decl = Declaration::<Texture2D, ()>::new();
            toast_decl
                .layout()
                .padding(Padding::new(16, 16, 8, 8))
                .direction(LayoutDirection::LeftToRight)
                .child_gap(12)
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end()
                .background_color(bg_color)
                .corner_radius()
                .all(8.0 * font_scale)
                .end();

            clay_scope.with(&toast_decl, |clay_scope| {
                let icon = match toast.toast_type {
                    ToastType::Info => ICON_TERMINAL,
                    ToastType::Warning => ICON_GAUGE,
                    ToastType::Error => ICON_MOVE,
                };

                clay_scope.text(
                    arena.push(format!("{}  {}", icon, toast.message)),
                    clay_layout::text::TextConfig::new()
                        .font_size((14.0 * font_scale) as u16)
                        .color(theme.cl_text_main)
                        .end(),
                );

                if let Some(ref action) = toast.action_label {
                    let action_btn_id = clay_scope.id(arena.push(format!("toast_action_{}", toast.id)));
                    let mut action_btn = Declaration::<Texture2D, ()>::new();
                    let mut btn_bg = theme.cl_bg_dark;
                    if clay_scope.pointer_over(action_btn_id) {
                        btn_bg = theme.cl_primary_hover;
                        if mouse_pressed {
                            action_ids.push(toast.id);
                        }
                    }
                    action_btn
                        .id(action_btn_id)
                        .layout()
                        .padding(Padding::new(8, 8, 4, 4))
                        .end()
                        .background_color(btn_bg)
                        .corner_radius()
                        .all(4.0 * font_scale)
                        .end();
                    clay_scope.with(&action_btn, |clay_scope| {
                        clay_scope.text(
                            arena.push(action.clone()),
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(theme.cl_text_main)
                                .end(),
                        );
                    });
                }

                if toast.has_dismiss {
                    let dismiss_btn_id = clay_scope.id(arena.push(format!("toast_dismiss_{}", toast.id)));
                    let mut dismiss_btn = Declaration::<Texture2D, ()>::new();
                    let mut btn_bg = theme.cl_bg_dark;
                    if clay_scope.pointer_over(dismiss_btn_id) {
                        btn_bg = theme.cl_danger;
                        if mouse_pressed {
                            dismiss_ids.push(toast.id);
                        }
                    }
                    dismiss_btn
                        .id(dismiss_btn_id)
                        .layout()
                        .padding(Padding::all(4))
                        .end()
                        .background_color(btn_bg)
                        .corner_radius()
                        .all(4.0 * font_scale)
                        .end();
                    clay_scope.with(&dismiss_btn, |clay_scope| {
                        clay_scope.text(
                            "X",
                            clay_layout::text::TextConfig::new()
                                .font_size((12.0 * font_scale) as u16)
                                .color(theme.cl_text_main)
                                .end(),
                        );
                    });
                }
            });
        }
    });

    if !dismiss_ids.is_empty() || !action_ids.is_empty() {
        let mut guard = state.lock().unwrap();
        for id in dismiss_ids {
            guard.active_toasts.retain(|t| t.id != id);
        }
        for id in action_ids {
            if let Some(toast) = guard.active_toasts.iter_mut().find(|t| t.id == id) {
                toast.action_clicked = true;
                toast.remaining_seconds = -1.0; // Mark for removal or handle action logic elsewhere
            }
        }
    }
}

pub fn render_tab_btn<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    active: bool,
    arena: &StringArena,
    font_scale: f32,
    theme: &Theme,
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
        theme.cl_bg_main
    } else {
        ClayColor::rgba(0.0, 0.0, 0.0, 0.0) // Transparent to show tab bar background
    };
    let mut text_color = if active {
        theme.cl_text_main
    } else {
        theme.cl_text_sub
    };

    if !active && clay.pointer_over(btn_id) {
        color = theme.cl_primary_hover;
        text_color = theme.cl_text_main;
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
        .all(0.0)
        .end()
        .border()
        .top((1.0 * font_scale) as u16)
        .left((1.0 * font_scale) as u16)
        .right((1.0 * font_scale) as u16)
        .color(theme.cl_bg_dark)
        .end();

    let mut clicked = false;
    clay.with(&btn, |clay_scope| {
        clay_scope.text(
            arena.push(format!("{}   {}", icon, label)),
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
    theme: &Theme,
) -> bool
where
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut color = if disabled {
        COLOR_BG_DISABLED
    } else {
        theme.cl_bg_section
    };
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = theme.cl_primary;
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
        theme.cl_text_main
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
    arena: &StringArena,
    font_scale: f32,
    disabled: bool,
    theme: &Theme,
) -> bool
where
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut color = if disabled {
        COLOR_BG_DISABLED
    } else {
        theme.cl_accent
    };
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = theme.cl_primary_hover; // or some light accent
        if mouse_pressed {
            clicked = true;
            let mut guard = state.lock().unwrap();
            guard.is_burning = true;
            guard.burn_log_active = true;
            let d = guard.distance;
            let f = guard.feed_rate;
            let s = guard.power;
            let dx_scaled = dx * d;
            let dy_scaled = dy * d;
            let cmd = format!(
                "{}\n{}\n{}\n{}",
                crate::gcode::CMD_RELATIVE_POS,
                crate::gcode::burn(dx_scaled, dy_scaled, s, f),
                crate::gcode::CMD_LASER_OFF,
                crate::gcode::CMD_HOME
            );
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
        .height(fixed!(theme.sz_btn_height * font_scale))
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
        theme.cl_text_main
    };
    clay.with(&btn, |clay| {
        clay.text(
            arena.push(format!("{}   {}", ICON_FLAME, label)),
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
    theme: &Theme,
) -> bool
where
    F: FnOnce() -> Option<String>,
    'a: 'render,
{
    let btn_id = clay.id(id);
    let mut color = if disabled {
        COLOR_BG_DISABLED
    } else {
        theme.cl_bg_dark
    };
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = theme.cl_primary_hover;
        if mouse_pressed {
            clicked = true;
            if let Some(gcode) = action() {
                if let Some((x, y, w, h)) = cli_and_helpers::get_gcode_bounds(&gcode) {
                    let mut guard = state.lock().unwrap();
                    guard.is_burning = true;
                    guard.burn_log_active = true;
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
        .width(fixed!(theme.sz_btn_height * font_scale))
        .height(fixed!(theme.sz_btn_height * font_scale))
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
        theme.cl_text_main
    };
    clay.with(&btn, |clay| {
        clay.text(
            ICON_SQUARE_VECTOR,
            clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(text_color).end(),
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
    theme: &Theme,
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
        .background_color(theme.cl_bg_section)
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
            theme.cl_primary
        } else {
            theme.cl_bg_dark
        };
        box_decl
            .layout()
            .width(fixed!(16.0 * font_scale))
            .height(fixed!(16.0 * font_scale))
            .direction(LayoutDirection::TopToBottom)
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
                        .color(theme.cl_text_main)
                        .end(),
                );
            }
        });

        clay_scope.text(
            label,
            clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(theme.cl_text_sub).end(),
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
    theme: &Theme,
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
            .child_gap(8)
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
            .end();
        clay.with(&header, |clay| {
            // Intelligent rounding
            let val_str = if value.fract() == 0.0 {
                format!("{}   {}", label, value as i32)
            } else {
                format!("{}   {:.1}", label, value)
            };
            clay.text(
                arena.push(val_str),
                clay_layout::text::TextConfig::new()
                    .font_size((14.0 * font_scale) as u16)
                    .color(theme.cl_text_label)
                    .end(),
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
            let mut minus_bg = theme.cl_bg_dark;
            if clay.pointer_over(btn_minus_id) {
                minus_bg = theme.cl_primary_hover;
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
                        .color(theme.cl_text_main)
                        .end(),
                );
            });

            // Track
            let mut track = Declaration::<Texture2D, ()>::new();
            track
                .id(slider_id)
                .layout()
                .width(grow!())
                .height(fixed!(16.0 * font_scale))
                .end()
                .background_color(theme.cl_bg_dark)
                .corner_radius()
                .all(4.0 * font_scale)
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
                    .background_color(color) // We keep custom slider color for now
                    .corner_radius()
                    .all(4.0 * font_scale)
                    .end();
                clay.with(&bar, |_| {});
            });

            // Plus Button
            let mut plus_box = Declaration::<Texture2D, ()>::new();
            let mut plus_bg = theme.cl_bg_dark;
            if clay.pointer_over(btn_plus_id) {
                plus_bg = theme.cl_primary_hover;
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
                        .color(theme.cl_text_main)
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
