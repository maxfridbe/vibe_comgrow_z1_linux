use clay_layout::{Declaration, Color, grow, fixed};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use crate::state::{AppState, StringArena};
use crate::ui::{Section, render_burn_btn};

pub fn render_test_left_col<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    sections: &[Section],
    mouse_pressed: bool,
    clipboard: &mut Option<Clipboard>,
    arena: &StringArena,
    font_scale: f32,
) where 'a: 'render {
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().height(grow!()).direction(clay_layout::layout::LayoutDirection::TopToBottom).child_gap(16).end();
    
    clay.with(&left_col, |clay_scope| {
        for section in sections {
            if section.title == "Test Patterns" {
                let mut section_box = Declaration::<Texture2D, ()>::new();
                section_box.layout().width(grow!()).direction(clay_layout::layout::LayoutDirection::TopToBottom).padding(clay_layout::layout::Padding::all(16)).child_gap(12).end()
                    .background_color(Color::u_rgb(30, 41, 59))
                    .corner_radius().all(16.0 * font_scale).end();
                
                clay_scope.with(&section_box, |clay| {
                    clay.text(section.title, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(section.color).end());
                    for cmd in &section.commands {
                        if render_burn_btn(clay, arena.push(format!("test_{}", cmd.label)), cmd.label, state, 0.0, 0.0, mouse_pressed, clipboard, font_scale) {
                            state.lock().unwrap().send_command(cmd.cmd.to_string());
                        }
                    }
                });
            }
        }
    });
}
