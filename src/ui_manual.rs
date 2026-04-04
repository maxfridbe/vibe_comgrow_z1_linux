use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::{Declaration, Color, grow, fixed, fit};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use crate::state::{AppState, StringArena};
use crate::ui::{Section, render_jog_btn, render_slider};

pub fn render_manual_left_col<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    sections: &[Section],
    mouse_pressed: bool,
    clipboard: &mut Option<Clipboard>,
    arena: &StringArena,
    font_scale: f32,
) where 'a: 'render {
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();
    clay.with(&left_col, |clay_scope| {
        for section in sections.iter().filter(|s| s.title != "Safety" && s.title != "Test Patterns") {
            let mut section_box = Declaration::<Texture2D, ()>::new();
            section_box.layout().width(grow!()).padding(Padding::all(6)).direction(LayoutDirection::TopToBottom).child_gap(12).end()
                .background_color(Color::u_rgb(30, 41, 59))
                .corner_radius().all(16.0 * font_scale).end();
            
            clay_scope.with(&section_box, |clay_scope| {
                let mut title_line = Declaration::<Texture2D, ()>::new();
                title_line.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center)).end();
                clay_scope.with(&title_line, |clay_scope| {
                    clay_scope.text(section.icon, clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(section.color).end());
                    clay_scope.text(section.title, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(100, 116, 139)).end());
                });

                for chunk in section.commands.chunks(2) {
                    let mut row = Declaration::<Texture2D, ()>::new();
                    row.layout().width(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).child_gap(8).end();
                    clay_scope.with(&row, |clay_scope| {
                        for cmd in chunk {
                            let btn_id = clay_scope.id(cmd.label);
                            let mut btn_color = Color::u_rgb(2, 6, 23);
                            if clay_scope.pointer_over(btn_id) {
                                btn_color = Color::u_rgb(51, 65, 85);
                                if mouse_pressed {
                                    let mut guard = state.lock().unwrap();
                                    guard.send_command(cmd.cmd.to_string());
                                }
                            }
                            let mut btn = Declaration::<Texture2D, ()>::new();
                            btn.id(btn_id).layout().width(fixed!(90.0 * font_scale)).padding(Padding::all(4)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end().background_color(btn_color).corner_radius().all(12.0 * font_scale).end();
                            clay_scope.with(&btn, |clay_scope| {
                                clay_scope.text(cmd.label, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                                clay_scope.text(cmd.cmd, clay_layout::text::TextConfig::new().font_size((10.0 * font_scale) as u16).color(Color::u_rgb(71, 85, 105)).end());
                            });
                        }
                    });
                }
            });
        }
    });
}

pub fn render_manual_right_col<'a, 'render>(
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
    let mut right_col = Declaration::<Texture2D, ()>::new();
    right_col.layout().height(grow!()).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top)).child_gap(16).width(fit!()).end();
    
    clay.with(&right_col, |clay_scope| {
        // Movement Box
        let mut move_box = Declaration::<Texture2D, ()>::new();
        move_box.layout().padding(Padding::all(12)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top)).child_gap(16).end().background_color(Color::u_rgb(30, 41, 59)).corner_radius().all(16.0 * font_scale).end();
        clay_scope.with(&move_box, |clay_scope| {
            let (dist, feed) = { let g = state.lock().unwrap(); (g.distance, g.feed_rate) };
            render_slider(clay_scope, "dist_slider", "Step", dist, 0.1, 100.0, Color::u_rgb(59, 130, 246), state, |s, v| s.distance = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
            render_slider(clay_scope, "feed_slider", "Speed", feed, 10.0, 6000.0, Color::u_rgb(16, 185, 129), state, |s, v| s.feed_rate = v, mouse_pos, mouse_down, scroll_y, arena, font_scale);
            
            let mut jog_grid = Declaration::<Texture2D, ()>::new();
            jog_grid.layout().child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).direction(LayoutDirection::TopToBottom).child_gap(8).end();
            clay_scope.with(&jog_grid, |clay_scope| {
                let mut row1 = Declaration::<Texture2D, ()>::new(); row1.layout().child_gap(8).end();
                clay_scope.with(&row1, |clay_scope| { render_jog_btn(clay_scope, "up", crate::icons::ICON_ARROW_UP, state, "Y", 1.0, mouse_pressed, clipboard, font_scale); });
                
                let mut row2 = Declaration::<Texture2D, ()>::new(); row2.layout().child_gap(8).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                clay_scope.with(&row2, |clay_scope| {
                    render_jog_btn(clay_scope, "left", crate::icons::ICON_ARROW_LEFT, state, "X", -1.0, mouse_pressed, clipboard, font_scale);
                    
                    let center_id = clay_scope.id("center");
                    let mut center_color = Color::u_rgb(0, 0, 0);
                    if clay_scope.pointer_over(center_id) {
                        center_color = Color::u_rgb(51, 65, 85);
                        if mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            guard.v_pos = raylib::prelude::Vector2::new(0.0, 0.0);
                            guard.send_command("G92 X0 Y0".to_string());
                            if let Some(cb) = clipboard { let _ = cb.set_text("G92 X0 Y0".to_string()); }
                        }
                    }
                    let mut center_btn = Declaration::<Texture2D, ()>::new();
                    center_btn.id(center_id).layout().width(fixed!(30.0 * font_scale)).height(fixed!(30.0 * font_scale)).padding(Padding::all(4)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                        .background_color(center_color).corner_radius().all(8.0 * font_scale).end();
                    clay_scope.with(&center_btn, |clay_scope| {
                        clay_scope.text(crate::icons::ICON_CROSSHAIR, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(59, 130, 246)).end());
                    });

                    let home_zero_id = clay_scope.id("home_zero");
                    let mut home_zero_color = Color::u_rgb(0, 0, 0);
                    if clay_scope.pointer_over(home_zero_id) {
                        home_zero_color = Color::u_rgb(51, 65, 85);
                        if mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            guard.v_pos = raylib::prelude::Vector2::new(0.0, 0.0);
                            guard.send_command("G90 G0 X0 Y0".to_string());
                            if let Some(cb) = clipboard { let _ = cb.set_text("G90 G0 X0 Y0".to_string()); }
                        }
                    }
                    let mut home_zero_btn = Declaration::<Texture2D, ()>::new();
                    home_zero_btn.id(home_zero_id).layout().width(fixed!(30.0 * font_scale)).height(fixed!(30.0 * font_scale)).padding(Padding::all(4)).direction(LayoutDirection::TopToBottom).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end()
                        .background_color(home_zero_color).corner_radius().all(8.0 * font_scale).end();
                    clay_scope.with(&home_zero_btn, |clay_scope| {
                        clay_scope.text(crate::icons::ICON_HOME, clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(52, 211, 153)).end());
                    });

                    render_jog_btn(clay_scope, "right", crate::icons::ICON_ARROW_RIGHT, state, "X", 1.0, mouse_pressed, clipboard, font_scale);
                });
                
                let mut row3 = Declaration::<Texture2D, ()>::new(); row3.layout().child_gap(8).end();
                clay_scope.with(&row3, |clay_scope| { render_jog_btn(clay_scope, "down", crate::icons::ICON_ARROW_DOWN, state, "Y", -1.0, mouse_pressed, clipboard, font_scale); });
            });
        });

        // Safety Section (minus E-STOP)
        if let Some(section) = sections.iter().find(|s| s.title == "Safety") {
            let mut safety_box = Declaration::<Texture2D, ()>::new();
            safety_box.layout().padding(Padding::all(12)).direction(LayoutDirection::TopToBottom).child_gap(16).end().background_color(Color::u_rgb(30, 41, 59)).corner_radius().all(16.0 * font_scale).end();
            clay_scope.with(&safety_box, |clay_scope| {
                clay_scope.text(section.title, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(section.color).end());
                for cmd in &section.commands {
                    let btn_id = clay_scope.id(cmd.label);
                    let mut btn_color = Color::u_rgb(2, 6, 23);
                    if clay_scope.pointer_over(btn_id) {
                        btn_color = Color::u_rgb(51, 65, 85);
                        if mouse_pressed { state.lock().unwrap().send_command(cmd.cmd.to_string()); }
                    }
                    let mut btn = Declaration::<Texture2D, ()>::new();
                    btn.id(btn_id).layout().width(fixed!(140.0 * font_scale)).padding(Padding::all(6)).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end().background_color(btn_color).corner_radius().all(12.0 * font_scale).end();
                    clay_scope.with(&btn, |clay_scope| {
                        clay_scope.text(cmd.label, clay_layout::text::TextConfig::new().font_size((12.0 * font_scale) as u16).color(Color::u_rgb(148, 163, 184)).end());
                    });
                }
            });
        }
    });
}
