use crate::state::AppState;
use crate::theme::Theme;
use raylib::prelude::*;
use std::sync::{Arc, Mutex};

fn draw_2d(
    d: &mut RaylibDrawHandle,
    g: &std::sync::MutexGuard<AppState>,
    draw_area: Rectangle,
    side: f32,
    _font_scale: f32,
    theme: &Theme,
    preview_texture: &RenderTexture2D,
) {
    // 1. Draw cached preview
    d.draw_texture_pro(
        preview_texture,
        Rectangle::new(0.0, 0.0, 2000.0, -2000.0),
        Rectangle::new(draw_area.x, draw_area.y, side, side),
        Vector2::new(0.0, 0.0),
        0.0,
        Color::WHITE,
    );

    // 2. Draw grid lines
    for i in 0..=20 {
        let offset = (i as f32 / 20.0) * side;
        let is_major = i % 5 == 0;
        let color = if is_major {
            Color::new(
                theme.cl_grid_major.r as u8,
                theme.cl_grid_major.g as u8,
                theme.cl_grid_major.b as u8,
                120, // Increased alpha
            )
        } else {
            Color::new(
                theme.cl_grid_minor.r as u8,
                theme.cl_grid_minor.g as u8,
                theme.cl_grid_minor.b as u8,
                60, // Increased alpha
            )
        };
        let thickness = if is_major { 2.0 } else { 1.0 };
        d.draw_line_ex(
            Vector2::new(draw_area.x + offset, draw_area.y),
            Vector2::new(draw_area.x + offset, draw_area.y + draw_area.height),
            thickness,
            color,
        );
        d.draw_line_ex(
            Vector2::new(draw_area.x, draw_area.y + offset),
            Vector2::new(draw_area.x + draw_area.width, draw_area.y + offset),
            thickness,
            color,
        );
    }

    // 3. Draw bounds
    if g.bounds.enabled {
        let bx = draw_area.x + (g.bounds.x / 400.0) * side;
        let by = draw_area.y + draw_area.height - (g.bounds.y / 400.0) * side - (g.bounds.h / 400.0) * side;
        d.draw_rectangle_lines_ex(
            Rectangle::new(
                bx,
                by,
                (g.bounds.w / 400.0) * side,
                (g.bounds.h / 400.0) * side,
            ),
            2.0,
            Color::new(
                theme.cl_bounds.r as u8,
                theme.cl_bounds.g as u8,
                theme.cl_bounds.b as u8,
                150,
            ),
        );
    }

    // 4. Draw real-time paths
    for p in &g.paths {
        let start = Vector2::new(
            draw_area.x + (p.x1 / 400.0) * side,
            draw_area.y + draw_area.height - (p.y1 / 400.0) * side,
        );
        let end = Vector2::new(
            draw_area.x + (p.x2 / 400.0) * side,
            draw_area.y + draw_area.height - (p.y2 / 400.0) * side,
        );
        d.draw_line_ex(
            start,
            end,
            2.0,
            Color::new(
                theme.cl_path.r as u8,
                theme.cl_path.g as u8,
                theme.cl_path.b as u8,
                (p.intensity * 255.0) as u8,
            ),
        );
    }

    // 5. Draw laser head
    let head_pos = Vector2::new(
        draw_area.x + (g.machine_pos.x / 400.0) * side,
        draw_area.y + draw_area.height - (g.machine_pos.y / 400.0) * side,
    );
    d.draw_circle_v(
        head_pos,
        5.0,
        Color::new(
            theme.cl_head.r as u8,
            theme.cl_head.g as u8,
            theme.cl_head.b as u8,
            100,
        ),
    );
    d.draw_circle_v(
        head_pos,
        2.0,
        Color::new(
            theme.cl_danger.r as u8,
            theme.cl_danger.g as u8,
            theme.cl_danger.b as u8,
            255,
        ),
    );
}

pub fn render_preview(
    d: &mut RaylibDrawHandle,
    state: &Arc<Mutex<AppState>>,
    interaction: &mut crate::ui_components::Interaction,
    draw_area: Rectangle,
    side: f32,
    font_scale: f32,
    theme: &Theme,
    preview_texture: &RenderTexture2D,
    delta_time: f32,
) {
    let mut g = state.lock().unwrap();

    if !interaction.is_handled {
        if interaction.mouse_pos.x >= draw_area.x
            && interaction.mouse_pos.x <= draw_area.x + draw_area.width
            && interaction.mouse_pos.y >= draw_area.y
            && interaction.mouse_pos.y <= draw_area.y + draw_area.height
        {
            if interaction.scroll_delta.y != 0.0 {
                interaction.is_handled = true;
                g.preview_zoom += interaction.scroll_delta.y * 0.1;
                if g.preview_zoom < 0.1 { g.preview_zoom = 0.1; }
                else if g.preview_zoom > 5.0 { g.preview_zoom = 5.0; }
            }

            let touch_count = unsafe { raylib::ffi::GetTouchPointCount() };
            if touch_count >= 2 {
                interaction.is_handled = true;
                let p0 = unsafe { raylib::ffi::GetTouchPosition(0) };
                let p1 = unsafe { raylib::ffi::GetTouchPosition(1) };
                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let dist = (dx * dx + dy * dy).sqrt();

                if g.touch_dist_prev > 0.0 {
                    let dist_delta = dist - g.touch_dist_prev;
                    g.preview_zoom += dist_delta * 0.005;
                    if g.preview_zoom < 0.1 { g.preview_zoom = 0.1; }
                    else if g.preview_zoom > 5.0 { g.preview_zoom = 5.0; }
                }
                g.touch_dist_prev = dist;
            } else if touch_count == 1 || interaction.mouse_down {
                interaction.is_handled = true;
                // Rotate camera around center when dragging left/right
                g.cam_orbit_angle += interaction.mouse_delta.x * 0.01;
            } else {
                g.touch_dist_prev = 0.0;
            }
        }
    }

    let speed = 2.0;
    if g.is_3d {
        g.anim_3d += delta_time * speed;
        if g.anim_3d > 1.0 { g.anim_3d = 1.0; }
    } else {
        g.anim_3d -= delta_time * speed;
        if g.anim_3d < 0.0 { g.anim_3d = 0.0; }
    }

    let anim_3d = g.anim_3d;
    let zoom_factor = g.preview_zoom;

    if anim_3d == 0.0 {
        draw_2d(d, &g, draw_area, side, font_scale, theme, preview_texture);
        return;
    }

    let is_burning = g.is_burning;
    let machine_pos = g.machine_pos;

    // Use ScissorMode to keep 3D contained
    let mut d_scissor = d.begin_scissor_mode(
        draw_area.x as i32,
        draw_area.y as i32,
        draw_area.width as i32,
        draw_area.height as i32,
    );

    // FIX: Use Viewport to center 3D camera in the draw area
    let screen_h = d_scissor.get_screen_height();
    unsafe {
        raylib::ffi::rlViewport(
            draw_area.x as i32,
            (screen_h as f32 - draw_area.y - draw_area.height) as i32,
            draw_area.width as i32,
            draw_area.height as i32
        );
    }

    let head_y_offset = -35.0;

    let target = Vector3::new(200.0, 200.0 + head_y_offset, 0.0);
    
    // Calculate rotated position based on orbit angle
    let angle = g.cam_orbit_angle;
    let dist_xy = 765.0;
    let cam_z = 600.0;
    
    // a=0 is looking from \"south\" (negative Y)
    let orbit_pos = Vector3::new(
        target.x + dist_xy * angle.sin(),
        target.y - dist_xy * angle.cos(),
        target.z + cam_z,
    );

    let start_pos = Vector3::new(200.0, 200.0 + head_y_offset, 600.0);
    let current_pos = start_pos.lerp(orbit_pos, anim_3d);
    let current_target = target;

    // Z is up.
    let up = Vector3::new(0.0, 0.0, 1.0);
    
    let current_fov = 60.0 / zoom_factor;
    let camera = Camera3D::perspective(
        current_pos,
        current_target,
        up,
        current_fov
    );

    {
        let mut d3d = d_scissor.begin_mode3D(camera);
        unsafe { 
            raylib::ffi::rlSetClipPlanes(0.1, 10000.0); 
            raylib::ffi::rlSetLineWidth(1.3);
        }

        unsafe {
            raylib::ffi::rlSetTexture(preview_texture.texture().id);
            raylib::ffi::rlBegin(raylib::ffi::RL_QUADS as i32);
            raylib::ffi::rlColor4ub(255, 255, 255, 255);
            
            raylib::ffi::rlTexCoord2f(0.0, 1.0);
            raylib::ffi::rlVertex3f(0.0, head_y_offset, 0.0);
            
            raylib::ffi::rlTexCoord2f(1.0, 1.0);
            raylib::ffi::rlVertex3f(400.0, head_y_offset, 0.0);
            
            raylib::ffi::rlTexCoord2f(1.0, 0.0);
            raylib::ffi::rlVertex3f(400.0, 400.0 + head_y_offset, 0.0);
            
            raylib::ffi::rlTexCoord2f(0.0, 0.0);
            raylib::ffi::rlVertex3f(0.0, 400.0 + head_y_offset, 0.0);
            
            raylib::ffi::rlEnd();
            raylib::ffi::rlSetTexture(0);
        }

        // Draw 3D Grid
        for i in 0..=20 {
            let offset = (i as f32 / 20.0) * 400.0;
            let is_major = i % 5 == 0;
            let color = if is_major {
                Color::new(theme.cl_grid_major.r as u8, theme.cl_grid_major.g as u8, theme.cl_grid_major.b as u8, 80)
            } else {
                Color::new(theme.cl_grid_minor.r as u8, theme.cl_grid_minor.g as u8, theme.cl_grid_minor.b as u8, 30)
            };
            d3d.draw_line_3D(Vector3::new(offset, head_y_offset, 0.0), Vector3::new(offset, 400.0 + head_y_offset, 0.0), color);
            d3d.draw_line_3D(Vector3::new(0.0, offset + head_y_offset, 0.0), Vector3::new(400.0, offset + head_y_offset, 0.0), color);
        }
        unsafe { raylib::ffi::rlSetLineWidth(1.0); }

        // Draw real-time paths
        for p in &g.paths {
            let start = Vector3::new(p.x1, p.y1 + head_y_offset, 0.0);
            let end = Vector3::new(p.x2, p.y2 + head_y_offset, 0.0);
            d3d.draw_line_3D(
                start,
                end,
                Color::new(theme.cl_path.r as u8, theme.cl_path.g as u8, theme.cl_path.b as u8, (p.intensity * 255.0) as u8)
            );
        }

        // Draw bounds
        if g.bounds.enabled {
            let bx = g.bounds.x;
            let by = g.bounds.y + head_y_offset;
            let bw = g.bounds.w;
            let bh = g.bounds.h;
            let bcolor = Color::new(theme.cl_bounds.r as u8, theme.cl_bounds.g as u8, theme.cl_bounds.b as u8, 150);
            d3d.draw_line_3D(Vector3::new(bx, by, 0.0), Vector3::new(bx + bw, by, 0.0), bcolor);
            d3d.draw_line_3D(Vector3::new(bx + bw, by, 0.0), Vector3::new(bx + bw, by + bh, 0.0), bcolor);
            d3d.draw_line_3D(Vector3::new(bx + bw, by + bh, 0.0), Vector3::new(bx, by + bh, 0.0), bcolor);
            d3d.draw_line_3D(Vector3::new(bx, by + bh, 0.0), Vector3::new(bx, by, 0.0), bcolor);
        }

        // --- Hardware Frame Fading In ---
        let hw_alpha = (anim_3d * 255.0) as u16;
        if hw_alpha > 0 {
            let black_alu = Color::new(35, 35, 35, hw_alpha as u8);
            let slot_color = Color::new(10, 10, 10, hw_alpha as u8);
            let blue_accent = Color::new(0, 128, 255, hw_alpha as u8);
            let rail_thickness = 20.0;
            
            // Helper to draw a \"2020 extrusion\" style rail
            let mut draw_rail = |d3d: &mut RaylibMode3D<RaylibScissorMode<RaylibDrawHandle>>, p: Vector3, w: f32, h: f32, l: f32| {
                d3d.draw_cube(p, w, h, l, black_alu);
                // Draw \"slots\" on the 4 longitudinal faces
                if w > h && w > l { // X-aligned
                    d3d.draw_cube(Vector3::new(p.x, p.y + h/2.0, p.z), w, 4.0, 4.0, slot_color);
                    d3d.draw_cube(Vector3::new(p.x, p.y - h/2.0, p.z), w, 4.0, 4.0, slot_color);
                    d3d.draw_cube(Vector3::new(p.x, p.y, p.z + l/2.0), w, 6.0, 4.0, slot_color);
                    d3d.draw_cube(Vector3::new(p.x, p.y, p.z - l/2.0), w, 6.0, 4.0, slot_color);
                } else if h > w && h > l { // Y-aligned
                    d3d.draw_cube(Vector3::new(p.x + w/2.0, p.y, p.z), 4.0, h, 4.0, slot_color);
                    d3d.draw_cube(Vector3::new(p.x - w/2.0, p.y, p.z), 4.0, h, 4.0, slot_color);
                    d3d.draw_cube(Vector3::new(p.x, p.y, p.z + l/2.0), 6.0, h, 4.0, slot_color);
                    d3d.draw_cube(Vector3::new(p.x, p.y, p.z - l/2.0), 6.0, h, 4.0, slot_color);
                }
            };

            // Outer frame rails - moved outward to clear 400x400 grid
            let frame_w = 510.0;
            let frame_l = 540.0;
            
            // X-axis rails (Front and back)
            draw_rail(&mut d3d, Vector3::new(200.0, -70.0, 10.0), frame_w, rail_thickness, rail_thickness);
            draw_rail(&mut d3d, Vector3::new(200.0, 450.0, 10.0), frame_w, rail_thickness, rail_thickness);

            // Y-axis rails (Left and right)
            draw_rail(&mut d3d, Vector3::new(-45.0, 190.0, 10.0), rail_thickness, frame_l, rail_thickness);
            draw_rail(&mut d3d, Vector3::new(445.0, 190.0, 10.0), rail_thickness, frame_l, rail_thickness);

            // Gantry Rail (moves along Y) - Spans Z=20 to 60
            let gantry_y = machine_pos.y;
            let gantry_width = 490.0;
            let gantry_z = 40.0;
            d3d.draw_cube(Vector3::new(200.0, gantry_y, gantry_z), gantry_width, 20.0, 40.0, black_alu);
            // Gantry slots
            d3d.draw_cube(Vector3::new(200.0, gantry_y + 10.0, gantry_z), gantry_width, 4.0, 6.0, slot_color);
            d3d.draw_cube(Vector3::new(200.0, gantry_y - 10.0, gantry_z), gantry_width, 4.0, 6.0, slot_color);
            d3d.draw_cube(Vector3::new(200.0, gantry_y, gantry_z + 20.0), gantry_width, 6.0, 4.0, slot_color);
            d3d.draw_cube(Vector3::new(200.0, gantry_y, gantry_z - 20.0), gantry_width, 6.0, 4.0, slot_color);

            // Y Motors (Blue Accents)
            d3d.draw_cylinder(Vector3::new(-45.0, -70.0, 15.0), 8.0, 8.0, 25.0, 16, blue_accent);
            d3d.draw_cylinder(Vector3::new(445.0, -70.0, 15.0), 8.0, 8.0, 25.0, 16, blue_accent);

            // X Motor (Blue Accent)
            d3d.draw_cylinder(Vector3::new(-45.0, gantry_y, gantry_z), 7.0, 7.0, 20.0, 16, blue_accent);

            // Laser Head (45x45x60 total)
            let head_x = machine_pos.x;
            let head_y = machine_pos.y;
            // Positioned in front of the gantry to avoid intersection
            let head_y_pos = head_y + head_y_offset; 
            
            // Black top part (30 units high)
            d3d.draw_cube(Vector3::new(head_x, head_y_pos, 45.0), 45.0, 45.0, 30.0, Color::new(45, 45, 45, hw_alpha as u8));
            // Translucent red shroud (30 units high)
            let shroud_color = Color::new(255, 0, 0, (hw_alpha as f32 * 0.4) as u8);
            d3d.draw_cube(Vector3::new(head_x, head_y_pos, 15.0), 45.0, 45.0, 30.0, shroud_color);
            
            // Laser head accent - raised to 55.5 to avoid Z-fighting with top face at 60.0
            d3d.draw_cube(Vector3::new(head_x, head_y_pos, 55.5), 47.0, 47.0, 10.0, blue_accent);

            // Burning Effect
            if is_burning {
                // Draw a tiny red beam from the center
                d3d.draw_cylinder(Vector3::new(head_x, head_y_pos, 0.0), 0.5, 0.5, 15.0, 8, Color::new(255, 0, 0, hw_alpha as u8));
                
                // Add some basic particles around the burn point
                let time = unsafe { raylib::ffi::GetTime() } as f32;
                for i in 0..8 {
                    let angle = time * 15.0 + (i as f32) * 0.8;
                    let radius = 1.0 + (time * 10.0 + i as f32).sin().abs() * 4.0;
                    let px = head_x + angle.cos() * radius;
                    let py = head_y_pos + angle.sin() * radius;
                    let pz = (time * 12.0 + i as f32 * 0.3).fract() * 8.0;
                    d3d.draw_cube(Vector3::new(px, py, pz), 1.2, 1.2, 1.2, Color::new(255, 150, 0, hw_alpha as u8));
                }
            }
        }
    }

    let sw = d_scissor.get_screen_width();
    let sh = d_scissor.get_screen_height();
    unsafe { raylib::ffi::rlViewport(0, 0, sw, sh); }
}
