use clay_layout::{Declaration, grow, fixed};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use crate::state::{AppState, StringArena};
use crate::ui::render_burn_btn;
use crate::styles::*;

pub fn render_svg_left_col<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    mouse_pressed: bool,
    clipboard: &mut Option<Clipboard>,
    _arena: &StringArena,
    font_scale: f32,
) where 'a: 'render {
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().height(grow!()).direction(clay_layout::layout::LayoutDirection::TopToBottom).child_gap(16).end();
    
    clay.with(&left_col, |clay_scope| {
        let mut svg_box = Declaration::<Texture2D, ()>::new();
        svg_box.layout().width(grow!()).direction(clay_layout::layout::LayoutDirection::TopToBottom).padding(clay_layout::layout::Padding::all(16)).child_gap(16).end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius().all(16.0 * font_scale).end();
        
        clay_scope.with(&svg_box, |clay| {
            clay.text("SVG Path Mode", clay_layout::text::TextConfig::new().font_size((20.0 * font_scale) as u16).color(COLOR_ACCENT_PURPLE_VIRTUAL).end());
            
            if render_burn_btn(clay, "btn_load_svg_ui", "Load SVG File", state, 0.0, 0.0, mouse_pressed, clipboard, font_scale, false) {
                println!("Load SVG Dialog Triggered");
            }

            // SVG Preview Area (Specific to SVG tab)
            let mut preview_box = Declaration::<Texture2D, ()>::new();
            preview_box.id(clay.id("svg_preview_canvas")).layout().width(grow!()).height(fixed!(300.0 * font_scale)).end()
                .background_color(COLOR_BG_MAIN)
                .corner_radius().all(12.0 * font_scale).end();
            
            clay.with(&preview_box, |clay| {
                clay.text("SVG PREVIEW", clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(COLOR_TEXT_LABEL).end());
            });
        });
    });
}
