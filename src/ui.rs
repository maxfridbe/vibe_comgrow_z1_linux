use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::{Declaration, Color, grow, fixed};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use crate::state::{AppState, StringArena, UITab};
use crate::icons::*;

pub fn render_tab_btn<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    active: bool,
    font_scale: f32,
) -> bool where 'a: 'render {
    let color = if active { Color::u_rgb(59, 130, 246) } else { Color::u_rgb(30, 41, 59) };
    let text_color = if active { Color::u_rgb(255, 255, 255) } else { Color::u_rgb(148, 163, 184) };
    
    let btn_id = clay.id(id);
    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id).layout().padding(Padding::new(16, 16, 10, 10)).end()
        .background_color(color)
        .corner_radius().top_left(8.0 * font_scale).top_right(8.0 * font_scale).end();
    
    let mut clicked = false;
    clay.with(&btn, |clay_scope| {
        clay_scope.text(label, clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(text_color).end());
        if unsafe { raylib::ffi::IsMouseButtonPressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT as i32) } && clay_scope.pointer_over(btn_id) {
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
    pub color: Color,
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
) -> bool where 'a: 'render {
    let btn_id = clay.id(id);
    let mut color = if disabled { Color::u_rgb(15, 23, 42) } else { Color::u_rgb(30, 41, 59) };
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = Color::u_rgb(59, 130, 246);
        if mouse_pressed {
            clicked = true;
            let mut guard = state.lock().unwrap();
            let d = guard.distance;
            let cmd = format!("$J=G91 G21 {}{} F{}", axis, direction * d, guard.feed_rate);
            guard.send_command(cmd.clone());
            if let Some(cb) = clipboard { let _ = cb.set_text(cmd); }
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
        .corner_radius().all(8.0 * font_scale).end();
    
    let text_color = if disabled { Color::u_rgb(71, 85, 105) } else { Color::u_rgb(255, 255, 255) };
    clay.with(&btn, |clay| {
        clay.text(icon, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(text_color).end());
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
) -> bool where 'a: 'render {
    let btn_id = clay.id(id);
    let mut color = if disabled { Color::u_rgb(15, 23, 42) } else { Color::u_rgb(147, 51, 234) }; 
    let mut clicked = false;
    if !disabled && clay.pointer_over(btn_id) {
        color = Color::u_rgb(168, 85, 247); 
        if mouse_pressed {
            clicked = true;
            let mut guard = state.lock().unwrap();
            let d = guard.distance;
            let f = guard.feed_rate;
            let s = guard.power;
            let dx_scaled = dx * d;
            let dy_scaled = dy * d;
            let cmd = format!("G91 G1 X{:.2} Y{:.2} F{} S{}", dx_scaled, dy_scaled, f, s);
            guard.send_command(cmd.clone());
            if let Some(cb) = clipboard { let _ = cb.set_text(cmd); }
        }
    }
    let mut btn = Declaration::<Texture2D, ()>::new();
    btn.id(btn_id)
        .layout()
            .width(fixed!(65.0 * font_scale))
            .padding(Padding::all(4))
            .direction(LayoutDirection::TopToBottom)
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
        .end()
        .background_color(color)
        .corner_radius().all(8.0 * font_scale).end();

    let text_color = if disabled { Color::u_rgb(71, 85, 105) } else { Color::u_rgb(255, 255, 255) };
    clay.with(&btn, |clay| {
        clay.text(label, clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(text_color).end());
    });
    clicked
}

pub fn render_slider<'a, 'render, F>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    id: &str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    color: Color,
    state: &Arc<Mutex<AppState>>,
    update: F,
    mouse_pos: raylib::math::Vector2,
    mouse_down: bool,
    scroll_y: f32,
    arena: &StringArena,
    font_scale: f32,
) where F: FnOnce(&mut AppState, f32), 'a: 'render {
    let slider_id = clay.id(id);
    let container_id = clay.id(arena.push(format!("{}_container", id)));
    let mut container = Declaration::<Texture2D, ()>::new();
    container.id(container_id)
        .layout().width(fixed!(180.0 * font_scale)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top)).child_gap(4).end();
    
    clay.with(&container, |clay| {
        let mut header = Declaration::<Texture2D, ()>::new();
        header.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
        clay.with(&header, |clay| {
            clay.text(label, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(100, 116, 139)).end());
            clay.text(arena.push(format!("{:.1}", value)), clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(color).end());
        });

        let mut track = Declaration::<Texture2D, ()>::new();
        track.id(slider_id).layout().width(grow!()).height(fixed!(6.0 * font_scale)).end()
            .background_color(Color::u_rgb(2, 6, 23))
            .corner_radius().all(3.0 * font_scale).end();
        
        if clay.pointer_over(slider_id) || clay.pointer_over(container_id) {
            if mouse_down {
                let data = unsafe { clay_layout::bindings::Clay_GetElementData(slider_id.id) };
                if data.found {
                    let rect = data.boundingBox;
                    let mouse_x = mouse_pos.x;
                    let percent = ((mouse_x - rect.x) / rect.width).clamp(0.0, 1.0);
                    let next = min + percent * (max - min);
                    let mut guard = state.lock().unwrap();
                    update(&mut guard, next);
                }
            } else if scroll_y != 0.0 {
                let step = (max - min) * 0.05;
                let next = (value + scroll_y * step).clamp(min, max);
                let mut guard = state.lock().unwrap();
                update(&mut guard, next);
            }
        }

        clay.with(&track, |clay| {
            let mut bar = Declaration::<Texture2D, ()>::new();
            let percent = (value - min) / (max - min);
            bar.layout().width(fixed!(percent * 180.0 * font_scale)).height(grow!()).end()
                .background_color(color)
                .corner_radius().all(3.0 * font_scale).end();
            clay.with(&bar, |_| {});
        });
    });
}
