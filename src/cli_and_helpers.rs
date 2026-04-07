use std::sync::mpsc;
use std::ffi::OsString;
use std::sync::Arc;
use crate::ui::Section;
use crate::svg_helper;
use crate::virtual_device::VirtualDevice;
use font_kit::source::SystemSource;
use font_kit::family_name::FamilyName;
use font_kit::properties::{Properties, Weight};
use font_kit::handle::Handle;
use font_kit::font::Font;
use font_kit::outline::OutlineSink;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_geometry::line_segment::LineSegment2F;

pub struct SafetyGuard {
    pub tx: mpsc::Sender<String>,
}

impl SafetyGuard {
    pub fn send_estop(&self) {
        println!("\n--- SAFETY: Sending Emergency Stop Sequence ---");
        let _ = self.tx.send("!".to_string());
        let _ = self.tx.send("M5".to_string());
        let _ = self.tx.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

impl Drop for SafetyGuard {
    fn drop(&mut self) {
        self.send_estop();
    }
}

fn parse_dimension(s: &str) -> Result<f32, Box<dyn std::error::Error + Send + Sync>> {
    let s = s.to_lowercase();
    if s.ends_with("in") {
        let val: f32 = s.trim_end_matches("in").parse()?;
        Ok(val * 25.4)
    } else if s.ends_with("mm") {
        let val: f32 = s.trim_end_matches("mm").parse()?;
        Ok(val)
    } else {
        let val: f32 = s.parse()?;
        Ok(val)
    }
}

pub fn run_cli_mode(target_label: &str, _sections: &[Section]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, _rx) = mpsc::channel::<String>();
    let _guard = SafetyGuard { tx: tx.clone() };
    
    let tx_ctrlc = tx.clone();
    ctrlc::set_handler(move || {
        println!("\n[CTRL-C] Detected.");
        let _ = tx_ctrlc.send("!".to_string());
        let _ = tx_ctrlc.send("M5".to_string());
        let _ = tx_ctrlc.send("0x18".to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
        std::process::exit(0);
    })?;

    println!("CLI Mode: {}", target_label);
    Ok(())
}

pub fn run_dynamic_pattern_cli(args: &[OsString]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut opts = pico_args::Arguments::from_vec(args.to_vec());
    let shape: String = opts.value_from_str("--shape")?;
    let pwr: String = opts.value_from_str("--power")?;
    let spd: String = opts.value_from_str("--speed")?;
    let scl: String = opts.value_from_str("--scale")?;
    let cx: f32 = opts.value_from_str("--cx")?;
    let cy: f32 = opts.value_from_str("--cy")?;
    let center: String = format!("{},{}", cx, cy);

    let (gcode, _) = generate_pattern_gcode(&shape, &pwr, &spd, &scl, "1", None, &center)?;
    println!("{}", gcode);
    Ok(())
}

fn parse_pair(s: &str) -> Result<(f32, f32), Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = s.split(|c| c == ',' || c == 'x').collect();
    if parts.len() < 2 { return Err("Invalid pair".into()); }
    Ok((parse_dimension(parts[0])?, parse_dimension(parts[1])?))
}

pub fn generate_pattern_gcode(shape: &str, pwr: &str, spd: &str, scale: &str, passes: &str, _fit: Option<String>, center: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr_val = pwr.trim_end_matches('%').parse::<f32>()?;
    let spd_val = spd.trim_end_matches('%').parse::<f32>()?;
    let scl_val = scale.trim_end_matches('x').parse::<f32>()?;
    let pas_val = passes.parse::<u32>().unwrap_or(1);
    let (cx, cy) = parse_pair(center)?;
    
    let mut final_gcode = String::new();
    final_gcode.push_str("G90\n$H\n");

    for _ in 0..pas_val {
        final_gcode.push_str(&format!("M4 S{} F{}\n", pwr_val, spd_val));
        match shape {
            "heart" => {
                let pts = 100;
                for i in 0..=pts {
                    let t = (i as f32 / pts as f32) * 2.0 * std::f32::consts::PI;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    final_gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", cx + x * scl_val, cy + y * scl_val));
                }
            }
            "star" => {
                let pts = 5;
                for i in 0..=(pts * 2) {
                    let angle = (i as f32 / (pts as f32 * 2.0)) * 2.0 * std::f32::consts::PI;
                    let r = if i % 2 == 0 { 20.0 } else { 8.0 };
                    let x = r * angle.cos();
                    let y = r * angle.sin();
                    final_gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", cx + x * scl_val, cy + y * scl_val));
                }
            }
            _ => {}
        }
        final_gcode.push_str("M5\n");
    }
    final_gcode.push_str("$H\n");

    Ok((final_gcode, format!("Pattern {} (Scale: {}x, Center: {}, Power: {}%, Speed: {}%)", shape, scl_val, center, pwr_val, spd_val)))
}

pub fn generate_image_gcode(path: &str, pwr_max: f32, speed: f32, scale: f32, passes: u32, fit: Option<(f32, f32)>, center: (f32, f32), low_fid: f32, high_fid: f32, is_preview: bool) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let mut img_base = image::open(path)?;
    
    // Resize image to a reasonable resolution for laser etching (e.g. max 500px dimension)
    // This dramatically speeds up G-code generation and reduces file size
    let max_dim = 500;
    if img_base.width() > max_dim || img_base.height() > max_dim {
        img_base = img_base.resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3);
    }
    let img = img_base.to_rgba8();
    
    let w = img.width() as f32;
    let h = img.height() as f32;
    
    let mut final_scale = scale;
    if let Some((fit_w, fit_h)) = fit {
        let sw = fit_w / w;
        let sh = fit_h / h;
        final_scale = sw.min(sh);
    }

    let out_w = w * final_scale;
    let out_h = h * final_scale;
    let offset_x = center.0 - out_w / 2.0;
    let offset_y = center.1 - out_h / 2.0;

    // Pre-allocate a large string to avoid reallocations
    let mut gcode = String::with_capacity(img.width() as usize * img.height() as usize * 40);
    gcode.push_str("G90\n$H\n");
    
    let f_val = (speed * 10.0) as i32;

    let effective_passes = if is_preview { 1 } else { passes };

    for _ in 0..effective_passes {
        for y in 0..img.height() {
            // Use 0.5 offset to center the laser on the pixel row
            let actual_y = offset_y + (img.height() as f32 - 0.5 - y as f32) * final_scale;

            // Find first and last non-zero pixels in this row to avoid crossing the whole canvas
            let mut first_x = None;
            let mut last_x = None;

            for x in 0..img.width() {
                let pixel = img.get_pixel(x, y);
                let luminance = 0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32;
                let intensity = 1.0 - (luminance / 255.0);
                let remapped = ((intensity - low_fid) / (high_fid - low_fid).max(0.001)).clamp(0.0, 1.0);

                if remapped > 0.01 { // Threshold for "empty"
                    if first_x.is_none() { first_x = Some(x); }
                    last_x = Some(x);
                }
            }

            if let (Some(fx), Some(lx)) = (first_x, last_x) {
                let left_to_right = y % 2 == 0;
                
                // Move to start of relevant content
                // If LTR, start at the LEFT edge of the first pixel (fx)
                // If RTL, start at the RIGHT edge of the last pixel (lx + 1)
                let start_x_coord = if left_to_right { fx as f32 } else { lx as f32 + 1.0 };
                gcode.push_str(&format!("M5\nG0 X{:.2} Y{:.2} F3000\n", offset_x + start_x_coord * final_scale, actual_y));
                gcode.push_str(&format!("M4 F{}\n", f_val));

                let range: Vec<u32> = if left_to_right {
                    (fx..=lx).collect()
                } else {
                    (fx..=lx).rev().collect()
                };

                for x in range {
                    let pixel = img.get_pixel(x, y);
                    let luminance = 0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32;
                    let intensity = 1.0 - (luminance / 255.0);
                    let remapped = ((intensity - low_fid) / (high_fid - low_fid).max(0.001)).clamp(0.0, 1.0);
                    let s_val = (remapped * pwr_max * 10.0) as i32;

                    // The destination coordinate
                    // If LTR, we move to the RIGHT edge of pixel x (x + 1)
                    // If RTL, we move to the LEFT edge of pixel x (x)
                    let dest_x_coord = if left_to_right { x as f32 + 1.0 } else { x as f32 };
                    let actual_x = offset_x + dest_x_coord * final_scale;

                    if s_val > 0 {
                        gcode.push_str(&format!("G1 X{:.2} S{}\n", actual_x, s_val));
                    } else {
                        // Internal jump over empty pixel
                        gcode.push_str(&format!("G0 X{:.2}\n", actual_x));
                    }
                }
            }
        }
    }
    gcode.push_str("M5\n$H\n");

    let filename = std::path::Path::new(path).file_name().and_then(|f| f.to_str()).unwrap_or("image");
    Ok((gcode, format!("Image {} (Scale: {:.2}x, Center: {:.1},{:.1}, Power: {}%, Speed: {}%, LowFid: {:.2}, HighFid: {:.2})", filename, final_scale, center.0, center.1, pwr_max, speed, low_fid, high_fid)))
}

struct VectorGCodeBuilder {
    gcode: String,
    offset: Vector2F,
    scale: f32,
    current_pos: Vector2F,
    start_pos: Vector2F,
    power: f32,
    speed: f32,
}

impl OutlineSink for VectorGCodeBuilder {
    fn move_to(&mut self, to: Vector2F) {
        let p = (to + self.offset) * self.scale;
        self.gcode.push_str(&format!("M5\nG0 X{:.2} Y{:.2} F3000\n", p.x(), p.y()));
        self.gcode.push_str(&format!("M4 S{} F{:.0}\n", self.power * 10.0, self.speed * 10.0));
        self.current_pos = to;
        self.start_pos = to;
    }

    fn line_to(&mut self, to: Vector2F) {
        let p = (to + self.offset) * self.scale;
        self.gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", p.x(), p.y()));
        self.current_pos = to;
    }

    fn quadratic_curve_to(&mut self, control: Vector2F, to: Vector2F) {
        let segments = 10;
        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let p = self.current_pos * (1.0 - t).powi(2) + control * 2.0 * (1.0 - t) * t + to * t.powi(2);
            self.line_to(p);
        }
    }

    fn cubic_curve_to(&mut self, control: LineSegment2F, to: Vector2F) {
        let segments = 10;
        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let p = self.current_pos * (1.0 - t).powi(3) + control.from() * 3.0 * (1.0 - t).powi(2) * t + control.to() * 3.0 * (1.0 - t) * t.powi(2) + to * t.powi(3);
            self.line_to(p);
        }
    }

    fn close(&mut self) {
        let d = (self.current_pos - self.start_pos).length();
        if d > 0.01 {
            self.line_to(self.start_pos);
        }
        self.gcode.push_str("M5\n");
    }
}

pub fn generate_text_gcode(text: &str, pwr_max: f32, speed: f32, scale: f32, passes: u32, fit: Option<(f32, f32)>, center: (f32, f32), bold: bool, outline: bool, letter_spacing: f32, _line_spacing: f32, font_family: &str, is_preview: bool) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    use font_kit::canvas::{Canvas, Format, RasterizationOptions};
    use font_kit::hinting::HintingOptions;
    use pathfinder_geometry::vector::Vector2I;
    use pathfinder_geometry::rect::RectF;
    use pathfinder_geometry::transform2d::Transform2F;

    let properties = Properties {
        weight: if bold { Weight::BOLD } else { Weight::NORMAL },
        ..Properties::new()
    };

    let font = if font_family == "Default" {
        Font::from_bytes(Arc::new(include_bytes!("../assets/font.ttf").to_vec()), 0).ok()
    } else {
        let source = SystemSource::new();
        source.select_best_match(&[FamilyName::Title(font_family.to_string())], &properties)
            .ok()
            .and_then(|handle| {
                match handle {
                    Handle::Path { path, font_index } => {
                        let bytes = std::fs::read(path).ok()?;
                        Font::from_bytes(Arc::new(bytes), font_index).ok()
                    }
                    Handle::Memory { bytes, font_index } => {
                        Font::from_bytes(bytes, font_index).ok()
                    }
                }
            })
    };

    let font = font.ok_or("Could not load font")?;
    let font_size = 64.0; 
    let units_per_em = font.metrics().units_per_em as f32;
    let design_to_px = font_size / units_per_em;

    // First pass: measure text in DESIGN UNITS (unscaled)
    let mut current_x = 0.0;
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    struct GlyphInfo {
        glyph_id: u32,
        offset: Vector2F,
    }
    let mut glyphs = Vec::new();

    for c in text.chars() {
        if let Some(glyph_id) = font.glyph_for_char(c) {
            let advance = font.advance(glyph_id).unwrap_or(Vector2F::new(0.0, 0.0)).x();
            let bounds = font.typographic_bounds(glyph_id).unwrap_or(RectF::new(Vector2F::new(0.0, 0.0), Vector2F::new(0.0, 0.0)));
            
            let gx = current_x + bounds.origin().x();
            let gy = bounds.origin().y();
            let gw = bounds.size().x();
            let gh = bounds.size().y();

            min_x = min_x.min(gx);
            max_x = max_x.max(gx + gw);
            min_y = min_y.min(gy);
            max_y = max_y.max(gy + gh);

            glyphs.push(GlyphInfo {
                glyph_id,
                offset: Vector2F::new(current_x, 0.0),
            });
            // letter_spacing is user units, scale it back to design units
            current_x += advance + (letter_spacing / (design_to_px * scale)).max(0.0);
        }
    }

    if glyphs.is_empty() {
        return Ok((String::new(), "Empty text".to_string()));
    }

    let mut final_user_scale = scale;
    if let Some((fw, fh)) = fit {
        let sw = fw / ((max_x - min_x) * design_to_px);
        let sh = fh / ((max_y - min_y) * design_to_px);
        final_user_scale = sw.min(sh);
    }
    let total_scale = design_to_px * final_user_scale;

    if outline {
        let mut gcode = String::new();
        gcode.push_str("G90\n$H\n");

        let effective_passes = if is_preview { 1 } else { passes };
        
        let box_center = Vector2F::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let center_vec = Vector2F::new(center.0, center.1);

        for _ in 0..effective_passes {
            for glyph in &glyphs {
                let mut builder = VectorGCodeBuilder {
                    gcode: String::new(),
                    offset: glyph.offset - box_center + (center_vec / total_scale),
                    scale: total_scale,
                    current_pos: Vector2F::new(0.0, 0.0),
                    start_pos: Vector2F::new(0.0, 0.0),
                    power: pwr_max,
                    speed: speed,
                };
                
                font.outline(glyph.glyph_id, HintingOptions::None, &mut builder).ok();
                gcode.push_str(&builder.gcode);
            }
        }
        gcode.push_str("$H\n");
        return Ok((gcode, format!("Text Outline \"{}\"", text)));
    }

    // Raster path
    let width = ((max_x - min_x) * design_to_px).max(1.0).ceil() as u32;
    let height = ((max_y - min_y) * design_to_px).max(1.0).ceil() as u32;
    let mut canvas = Canvas::new(Vector2I::new(width as i32, height as i32), Format::A8);

    for glyph in glyphs {
        let origin = Vector2F::new((glyph.offset.x() - min_x) * design_to_px, max_y * design_to_px);
        font.rasterize_glyph(
            &mut canvas,
            glyph.glyph_id,
            font_size,
            Transform2F::from_translation(origin),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        ).ok();
    }

    // Convert A8 canvas to PNG in memory
    let mut rgba_pixels = Vec::with_capacity((width * height * 4) as usize);
    for pixel in canvas.pixels {
        // We want black text on white background for the gcode generator
        let intensity = 255 - pixel;
        rgba_pixels.push(intensity); // R
        rgba_pixels.push(intensity); // G
        rgba_pixels.push(intensity); // B
        rgba_pixels.push(255);       // A
    }

    let img_buffer = image::RgbaImage::from_raw(width, height, rgba_pixels).ok_or("Failed to create image buffer")?;
    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);
    img_buffer.write_to(&mut cursor, image::ImageFormat::Png)?;

    let img = raylib::prelude::Image::load_image_from_mem(".png", &png_data).map_err(|e| e.to_string())?;

    let temp_path = "temp_text_render.png";
    img.export_image(temp_path);

    let result = generate_image_gcode(temp_path, pwr_max, speed, final_user_scale, passes, None, center, 0.0, 1.0, is_preview);
    let _ = std::fs::remove_file(temp_path);
    result
}

pub fn generate_outline_gcode(x: f32, y: f32, w: f32, h: f32, speed: f32) -> String {
    format!(
        "$H\nG90\nM5\nG0 X{:.2} Y{:.2} F3000\nG0 X{:.2} Y{:.2} F{:.0}\nG0 X{:.2} Y{:.2}\nG0 X{:.2} Y{:.2}\nG0 X{:.2} Y{:.2}\nM5\n$H",
        x, y,
        x + w, y, speed,
        x + w, y + h,
        x, y + h,
        x, y
    )
}

pub fn get_gcode_bounds(gcode: &str) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    let mut curr_x = 0.0;
    let mut curr_y = 0.0;
    let mut found = false;

    for line in gcode.lines() {
        let line = line.trim().to_uppercase();
        if line.starts_with('G') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut x = None;
            let mut y = None;
            for p in parts {
                if p.starts_with('X') { x = p[1..].parse::<f32>().ok(); }
                if p.starts_with('Y') { y = p[1..].parse::<f32>().ok(); }
            }
            if let Some(val) = x { curr_x = val; }
            if let Some(val) = y { curr_y = val; }
            
            if x.is_some() || y.is_some() {
                min_x = min_x.min(curr_x);
                max_x = max_x.max(curr_x);
                min_y = min_y.min(curr_y);
                max_y = max_y.max(curr_y);
                found = true;
            }
        }
    }

    if found {
        Some((min_x, min_y, max_x - min_x, max_y - min_y))
    } else {
        None
    }
}

pub fn run_serial_cmd(cmd_str: &str, label: &str, _tx: mpsc::Sender<String>, use_virtual: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::io::{Write, Read};
    use crate::gcode::{decode_response, decode_gcode};

    fn get_ts() -> String {
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();
        let hh = (secs / 3600) % 24;
        let mm = (secs / 60) % 60;
        let ss = secs % 60;
        format!("{:02}:{:02}:{:02}", hh, mm, ss)
    }

    if use_virtual {
        println!("[{}] VIRTUAL: {} -> {}", get_ts(), label, cmd_str);
    } else {
        println!("[{}] SENDING: {}...", get_ts(), label);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_gcode_generation_not_empty() {
        let text = "Hi";
        let pwr = 100.0;
        let spd = 1000.0;
        let scl = 1.0;
        let passes = 1;
        let fit = None;
        let center = (200.0, 200.0);
        let bold = false;
        let _outline = false;
        let letter_spacing = 0.0;
        let _line_spacing = 1.0;
        let font_family = "Default";

        let result = generate_text_gcode(text, pwr, spd, scl, passes, fit, center, bold, _outline, letter_spacing, _line_spacing, font_family, true);
        
        match result {
            Ok((gcode, label)) => {
                println!("GCode generated length: {}", gcode.len());
                assert!(!gcode.is_empty(), "G-code should not be empty");
                assert!(gcode.contains("G1"), "G-code should contain movement commands");
                assert!(label.contains("Image"), "Label should describe the generated text");
            },
            Err(e) => panic!("G-code generation failed: {}", e),
        }
    }

    #[test]
    fn test_text_gcode_generation_outline() {
        let text = "Hi";
        let pwr = 100.0;
        let spd = 1000.0;
        let scl = 1.0;
        let passes = 1;
        let fit = None;
        let center = (200.0, 200.0);
        let bold = false;
        let outline = true;
        let letter_spacing = 0.0;
        let _line_spacing = 1.0;
        let font_family = "Default";

        let result = generate_text_gcode(text, pwr, spd, scl, passes, fit, center, bold, outline, letter_spacing, _line_spacing, font_family, true);
        
        match result {
            Ok((gcode, label)) => {
                println!("Outline GCode length: {}", gcode.len());
                assert!(!gcode.is_empty(), "G-code should not be empty");
                assert!(gcode.contains("G1"), "G-code should contain movement commands");
                assert!(label.contains("Outline"), "Label should describe the outline mode");
            },
            Err(e) => panic!("Outline G-code generation failed: {}", e),
        }
    }
}
