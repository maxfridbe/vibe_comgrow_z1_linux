use crate::gcode;
use crate::svg_helper;
use crate::ui::Section;
use crate::virtual_device::VirtualDevice;
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::handle::Handle;
use font_kit::outline::OutlineSink;
use font_kit::properties::{Properties, Weight};
use font_kit::source::SystemSource;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::vector::Vector2F;
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use crate::state::{AppState, SavedState, ImageBurnConfig, TextBurnConfig, BurnConfig, Bounds};
use std::fmt::Write;
use std::sync::mpsc;

pub struct SafetyGuard {
    pub tx: mpsc::Sender<String>,
}

impl SafetyGuard {
    pub fn send_estop(&self) {
        println!("\n--- SAFETY: Sending Emergency Stop Sequence ---");
        let _ = self.tx.send(gcode::CMD_FEED_HOLD.to_string());
        let _ = self.tx.send(gcode::CMD_LASER_OFF.to_string());
        let _ = self.tx.send(gcode::CMD_SOFT_RESET.to_string());
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

impl Drop for SafetyGuard {
    fn drop(&mut self) {
        self.send_estop();
    }
}

fn get_ts() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let hh = (secs / 3600) % 24;
    let mm = (secs / 60) % 60;
    let ss = secs % 60;
    format!("{:02}:{:02}:{:02}", hh, mm, ss)
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
    let home = std::env::var("HOME")?;
    let path = std::path::PathBuf::from(home).join(".config").join("trogdor").join("saved_states.json");
    if let Ok(json) = std::fs::read_to_string(path) {
        let states: Vec<SavedState> = serde_json::from_str(&json)?;
        if let Some(state) = states.iter().find(|s| s.label == target_label) {
            println!("[{}] Found saved state: {}", get_ts(), state.label);
            return Ok(());
        }
    }
    println!("[{}] Error: State '{}' not found.", get_ts(), target_label);
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
    let config = BurnConfig {
        power: pwr.trim_end_matches('%').parse::<f32>()? * 10.0,
        feed_rate: spd.trim_end_matches('%').parse::<f32>()? * 10.0,
        scale: scl.trim_end_matches('x').parse::<f32>()?,
        passes: 1,
        bounds: Bounds { enabled: true, x: cx - 200.0, y: cy - 200.0, w: 400.0, h: 400.0 },
    };

    let (gcode_str, label) = generate_pattern_gcode(&shape, &config, false)?;
    println!("[{}] Generated: {}\n{}", get_ts(), label, gcode_str);
    Ok(())
}

pub fn generate_pattern_gcode(
    shape: &str,
    config: &BurnConfig,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr_val = config.power;
    let spd_val = if is_preview { config.feed_rate.min(1000.0) } else { config.feed_rate };
    let scl_val = config.scale;
    let pas_val = config.passes;
    let cx = if config.bounds.enabled { config.bounds.x + config.bounds.w / 2.0 } else { 200.0 };
    let cy = if config.bounds.enabled { config.bounds.y + config.bounds.h / 2.0 } else { 200.0 };

    let car_svg = include_str!("../assets/car.svg");
    let stars8_svg = include_str!("../assets/stars8.svg");
    let stars9_svg = include_str!("../assets/stars9.svg");
    let star_svg = include_str!("../assets/star.svg");

    let mut final_gcode = String::new();
    final_gcode.push_str(&format!("{}\n{}\n", gcode::CMD_ABSOLUTE_POS, gcode::CMD_HOME));

    for _ in 0..pas_val {
        let shape_lower = shape.to_lowercase();
        match shape_lower.as_str() {
            "square" => {
                let s = 20.0 * scl_val;
                let x1 = (cx - s).clamp(0.0, 400.0);
                let y1 = (cy - s).clamp(0.0, 400.0);
                let x2 = (cx + s).clamp(0.0, 400.0);
                let y2 = (cy + s).clamp(0.0, 400.0);
                final_gcode.push_str(&format!("{}\n{}\n", gcode::CMD_LASER_OFF, gcode::move_xy(x1, y1)));
                final_gcode.push_str(&format!("{} F{}\n", gcode::laser_on_dynamic(pwr_val), spd_val));
                final_gcode.push_str(&format!("{}\n", gcode::burn_s(x2, y1, pwr_val)));
                final_gcode.push_str(&format!("{}\n", gcode::burn_s(x2, y2, pwr_val)));
                final_gcode.push_str(&format!("{}\n", gcode::burn_s(x1, y2, pwr_val)));
                final_gcode.push_str(&format!("{}\n", gcode::burn_s(x1, y1, pwr_val)));
            }
            "heart" => {
                let pts = 100;
                for i in 0..=pts {
                    let t = (i as f32 / pts as f32) * 2.0 * std::f32::consts::PI;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    let px_raw = cx + x * scl_val;
                    let py_raw = cy + y * scl_val;
                    let px = px_raw.clamp(0.0, 400.0);
                    let py = py_raw.clamp(0.0, 400.0);
                    let p_val = if px_raw < 0.0 || px_raw > 400.0 || py_raw < 0.0 || py_raw > 400.0 { 0.0 } else { pwr_val };
                    if i == 0 {
                        final_gcode.push_str(&format!("{}\n{}\n", gcode::CMD_LASER_OFF, gcode::move_xy(px, py)));
                        final_gcode.push_str(&format!("{} F{}\n", gcode::laser_on_dynamic(p_val), spd_val));
                    } else {
                        final_gcode.push_str(&format!("{}\n", gcode::burn_s(px, py, p_val)));
                    }
                }
            }
            "star" | "stars8" | "stars9" | "car" => {
                let svg_data = match shape_lower.as_str() {
                    "star" => star_svg, "stars8" => stars8_svg, "stars9" => stars9_svg, "car" => car_svg, _ => unreachable!(),
                };
                let fit = if config.bounds.enabled { Some((config.bounds.w, config.bounds.h)) } else { None };
                if let Ok((svg_gcode, _, _, _, _)) = svg_helper::load_svg_data_as_gcode(
                    svg_data.as_bytes(),
                    scl_val,
                    fit,
                    cx,
                    cy,
                    pwr_val as i32,
                    spd_val as i32,
                ) {
                    final_gcode.push_str(&svg_gcode);
                }
            }
            _ => {
                if shape_lower.ends_with(".svg") {
                    let fit = if config.bounds.enabled { Some((config.bounds.w, config.bounds.h)) } else { None };
                    if let Ok((svg_gcode, _, _, _, _)) = svg_helper::load_svg_as_gcode(
                        shape,
                        scl_val,
                        fit,
                        cx,
                        cy,
                        pwr_val as i32,
                        spd_val as i32,
                    ) {
                        final_gcode.push_str(&svg_gcode);
                    }
                }
            }
        }
    }
    final_gcode.push_str(&format!("{}\n{}\n", gcode::CMD_LASER_OFF, gcode::CMD_HOME));
    Ok((final_gcode, format!("Pattern {} (Scale: {}x, Center: {:.1},{:.1})", shape, scl_val, cx, cy)))
}

pub fn generate_image_gcode(
    path: &str,
    config: &ImageBurnConfig,
    intensity_override: Option<f32>,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr_max = config.base.power;
    let speed = if is_preview { config.base.feed_rate.min(1000.0) } else { config.base.feed_rate };
    let low_fid = config.low_fid;
    let high_fid = config.high_fid;
    let lines_per_mm = config.lines_per_mm;
    let cx = if config.base.bounds.enabled { config.base.bounds.x + config.base.bounds.w / 2.0 } else { 200.0 };
    let cy = if config.base.bounds.enabled { config.base.bounds.y + config.base.bounds.h / 2.0 } else { 200.0 };

    println!("[{}] Opening image: {}", get_ts(), path);
    let img_base = image::open(path)?;
    let (orig_w, orig_h) = (img_base.width() as f32, img_base.height() as f32);

    let mut final_scale = config.base.scale;
    if config.base.bounds.enabled {
        final_scale = (config.base.bounds.w / orig_w).min(config.base.bounds.h / orig_h);
    }

    let out_w = orig_w * final_scale;
    let out_h = orig_h * final_scale;

    // Hard cap internal resolution at 2000px to prevent OOM and extreme processing times.
    // We calculate the target dimensions based on lines_per_mm but then scale down to fit max_px.
    let max_px = 2000.0;
    let target_w_raw = out_w * lines_per_mm;
    let target_h_raw = out_h * lines_per_mm;
    
    let res_scale = (max_px / target_w_raw.max(target_h_raw)).min(1.0);
    let target_w = (target_w_raw * res_scale).max(1.0).ceil() as u32;
    let target_h = (target_h_raw * res_scale).max(1.0).ceil() as u32;
    
    let working_img = if img_base.width() > target_w || img_base.height() > target_h {
        println!("[{}] Pre-shrinking image to {}x{}...", get_ts(), target_w, target_h);
        img_base.thumbnail(target_w, target_h)
    } else {
        img_base
    };

    println!("[{}] Resizing to final resolution: {}x{}", get_ts(), target_w, target_h);
    let img = working_img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3).to_rgba8();
    
    let width = img.width();
    let height = img.height();
    
    // Physical size of one G-code pixel step in mm
    let pixel_scale_x = out_w / width as f32;
    let pixel_scale_y = out_h / height as f32;
    
    let offset_x = cx - out_w / 2.0;
    let offset_y = cy - out_h / 2.0;

    let img_slice = img.as_flat_samples();
    let pixels = img_slice.as_slice();

    // Fast pre-calculation of row data
    let mut row_intensities = vec![Vec::with_capacity(width as usize); height as usize];
    let mut row_has_data = vec![false; height as usize];
    for y in 0..height {
        let row_start = (y * width * 4) as usize;
        for x in 0..width {
            let idx = row_start + (x * 4) as usize;
            let luminance = 0.2126 * pixels[idx] as f32 + 0.7152 * pixels[idx+1] as f32 + 0.0722 * pixels[idx+2] as f32;
            let intensity = 1.0 - (luminance / 255.0);
            let remapped = ((intensity - low_fid) / (high_fid - low_fid).max(0.001)).clamp(0.0, 1.0);
            row_intensities[y as usize].push(remapped);
            if remapped > 0.01 { row_has_data[y as usize] = true; }
        }
    }

    let mut gcode_out = String::with_capacity((width * height) as usize * 15);
    gcode_out.push_str(&format!("{}\n{}\n", gcode::CMD_ABSOLUTE_POS, gcode::CMD_HOME));

    println!("[{}] Starting G-code generation...", get_ts());
    let start_time = std::time::Instant::now();
    let mut row_count = 0;
    let f_val = (speed * 10.0) as i32;

    for _ in 0..if is_preview { 1 } else { config.base.passes } {
        for y in 0..height {
            if !row_has_data[y as usize] { continue; }
            row_count += 1;
            let actual_y = (offset_y + (height as f32 - 0.5 - y as f32) * pixel_scale_y).clamp(0.0, 400.0);
            let row = &row_intensities[y as usize];
            
            let mut first_x = None;
            let mut last_x = None;
            for (x, &val) in row.iter().enumerate() {
                if val > 0.01 {
                    if first_x.is_none() { first_x = Some(x); }
                    last_x = Some(x);
                }
            }

            if let (Some(fx), Some(lx)) = (first_x, last_x) {
                let left_to_right = y % 2 == 0;
                let start_x = if left_to_right { fx as f32 } else { lx as f32 + 1.0 };
                let px = (offset_x + start_x * pixel_scale_x).clamp(0.0, 400.0);
                
                gcode_out.push_str(&format!("{}\n{}\n{}\n", gcode::CMD_LASER_OFF, gcode::move_xy_f(px, actual_y, 3000.0), gcode::laser_dynamic_f_only(f_val as f32)));

                if left_to_right {
                    let mut x = fx;
                    while x <= lx {
                        let val = row[x];
                        let s_val = if let Some(override_val) = intensity_override {
                            if val > 0.01 { override_val as i32 } else { 0 }
                        } else {
                            (val * pwr_max) as i32
                        };
                        // Find consecutive pixels with same intensity
                        let mut end_x = x;
                        while end_x < lx {
                            let next_val = row[end_x + 1];
                            let next_s = if let Some(override_val) = intensity_override {
                                if next_val > 0.01 { override_val as i32 } else { 0 }
                            } else {
                                (next_val * pwr_max) as i32
                            };
                            if next_s != s_val { break; }
                            end_x += 1;
                        }
                        let dest_x = offset_x + (end_x as f32 + 1.0) * pixel_scale_x;
                        let actual_x = dest_x.clamp(0.0, 400.0);
                        if s_val > 0 && dest_x >= 0.0 && dest_x <= 400.0 {
                            writeln!(gcode_out, "{}", gcode::burn_xs(actual_x, s_val as f32)).ok();
                        } else {
                            writeln!(gcode_out, "{}", gcode::burn_xs(actual_x, 0.0)).ok();
                        }
                        x = end_x + 1;
                    }
                } else {
                    let mut x = lx;
                    while x >= fx {
                        let val = row[x];
                        let s_val = if let Some(override_val) = intensity_override {
                            if val > 0.01 { override_val as i32 } else { 0 }
                        } else {
                            (val * pwr_max) as i32
                        };
                        let mut end_x = x;
                        while end_x > fx {
                            let next_val = row[end_x - 1];
                            let next_s = if let Some(override_val) = intensity_override {
                                if next_val > 0.01 { override_val as i32 } else { 0 }
                            } else {
                                (next_val * pwr_max) as i32
                            };
                            if next_s != s_val { break; }
                            end_x -= 1;
                        }
                        let dest_x = offset_x + (end_x as f32) * pixel_scale_x;
                        let actual_x = dest_x.clamp(0.0, 400.0);
                        if s_val > 0 && dest_x >= 0.0 && dest_x <= 400.0 {
                            writeln!(gcode_out, "{}", gcode::burn_xs(actual_x, s_val as f32)).ok();
                        } else {
                            writeln!(gcode_out, "{}", gcode::burn_xs(actual_x, 0.0)).ok();
                        }
                        if end_x == 0 { break; }
                        x = end_x - 1;
                    }
                }
            }
        }
    }
    gcode_out.push_str(&format!("{}\n{}\n", gcode::CMD_LASER_OFF, gcode::CMD_HOME));
    println!("[{}] Finished generation. Rows: {}, Duration: {:?}, GCode Length: {}", get_ts(), row_count, start_time.elapsed(), gcode_out.len());
    Ok((gcode_out, format!("Image {} ({:.1}x, Power: {}%, Speed: {}%)", std::path::Path::new(path).file_name().and_then(|f| f.to_str()).unwrap_or("image"), final_scale, pwr_max, speed)))
}

pub fn generate_text_gcode(
    config: &TextBurnConfig,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr_max = config.base.power;
    let speed = config.base.feed_rate;
    let scale = config.base.scale;
    let text = &config.content;
    let bold = config.is_bold;
    let outline = config.is_outline;
    let letter_spacing = config.letter_spacing;
    let curve_steps = config.curve_steps;
    let lines_per_mm = config.lines_per_mm;
    let font_family = &config.font;
    let cx = if config.base.bounds.enabled { config.base.bounds.x + config.base.bounds.w / 2.0 } else { 200.0 };
    let cy = if config.base.bounds.enabled { config.base.bounds.y + config.base.bounds.h / 2.0 } else { 200.0 };

    use font_kit::canvas::{Canvas, Format, RasterizationOptions};
    use font_kit::hinting::HintingOptions;
    use pathfinder_geometry::rect::RectF;
    use pathfinder_geometry::transform2d::Transform2F;
    use pathfinder_geometry::vector::Vector2I;

    let properties = Properties { weight: if bold { Weight::BOLD } else { Weight::NORMAL }, ..Properties::new() };
    let font = if font_family == "Default" {
        Font::from_bytes(Arc::new(include_bytes!("../assets/font.ttf").to_vec()), 0).ok()
    } else {
        let source = SystemSource::new();
        source.select_best_match(&[FamilyName::Title(font_family.to_string())], &properties).ok().and_then(|handle| match handle {
            Handle::Path { path, font_index } => { let bytes = std::fs::read(path).ok()?; Font::from_bytes(Arc::new(bytes), font_index).ok() }
            Handle::Memory { bytes, font_index } => Font::from_bytes(bytes, font_index).ok(),
        })
    };
    let font = font.ok_or("Could not load font")?;
    let font_size = 100.0;
    let units_per_em = font.metrics().units_per_em as f32;
    let design_to_px = font_size / units_per_em;

    let mut current_x = 0.0;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    struct GlyphInfo { glyph_id: u32, offset: Vector2F }
    let mut glyphs = Vec::new();

    for c in text.chars() {
        if let Some(glyph_id) = font.glyph_for_char(c) {
            let advance = font.advance(glyph_id).unwrap_or(Vector2F::new(0.0, 0.0)).x();
            let bounds = font.typographic_bounds(glyph_id).unwrap_or(RectF::new(Vector2F::new(0.0, 0.0), Vector2F::new(0.0, 0.0)));
            let gx = current_x + bounds.origin().x();
            let gy = bounds.origin().y();
            min_x = min_x.min(gx); max_x = max_x.max(gx + bounds.size().x());
            min_y = min_y.min(gy); max_y = max_y.max(gy + bounds.size().y());
            glyphs.push(GlyphInfo { glyph_id, offset: Vector2F::new(current_x, 0.0) });
            current_x += advance + (letter_spacing / (design_to_px * scale)).max(0.0);
        }
    }
    if glyphs.is_empty() { return Ok((String::new(), "Empty text".to_string())); }

    let mut final_user_scale = scale;
    if config.base.bounds.enabled {
        final_user_scale = (config.base.bounds.w / ((max_x - min_x) * design_to_px)).min(config.base.bounds.h / ((max_y - min_y) * design_to_px));
    }
    let total_scale = design_to_px * final_user_scale;

    if outline {
        let mut gcode_out = String::new();
        gcode_out.push_str(&format!("{}\n{}\n", gcode::CMD_ABSOLUTE_POS, gcode::CMD_HOME));
        let box_center = Vector2F::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        for _ in 0..if is_preview { 1 } else { config.base.passes } {
            for glyph in &glyphs {
                let mut builder = VectorGCodeBuilder {
                    gcode: String::new(),
                    offset: glyph.offset - box_center + (Vector2F::new(cx, cy) / total_scale),
                    scale: total_scale,
                    current_pos: Vector2F::new(0.0, 0.0),
                    start_pos: Vector2F::new(0.0, 0.0),
                    power: pwr_max, speed, curve_steps,
                };
                font.outline(glyph.glyph_id, HintingOptions::None, &mut builder).ok();
                gcode_out.push_str(&builder.gcode);
            }
        }
        gcode_out.push_str(&format!("{}\n{}\n", gcode::CMD_LASER_OFF, gcode::CMD_HOME));
        return Ok((gcode_out, format!("Text Outline \"{}\"", text)));
    }

    let out_h = (max_y - min_y) * total_scale;
    let target_pixels_h = (out_h * lines_per_mm).max(1.0).ceil() as u32;
    let target_font_size = (target_pixels_h as f32) / ((max_y - min_y) / units_per_em);
    let target_design_to_px = target_font_size / units_per_em;
    let width = ((max_x - min_x) * target_design_to_px).max(1.0).ceil() as u32;
    let mut canvas = Canvas::new(Vector2I::new(width as i32, target_pixels_h as i32), Format::A8);

    for glyph in glyphs {
        let origin = Vector2F::new((glyph.offset.x() - min_x) * target_design_to_px, max_y * target_design_to_px);
        font.rasterize_glyph(&mut canvas, glyph.glyph_id, target_font_size, Transform2F::from_translation(origin), HintingOptions::None, RasterizationOptions::GrayscaleAa).ok();
    }

    let temp_path = "temp_text_render.png";
    let mut img_buffer = image::ImageBuffer::new(width, target_pixels_h);
    for (x, y, pixel) in img_buffer.enumerate_pixels_mut() {
        let val = canvas.pixels[(y * width + x) as usize];
        let inv = 255 - val;
        *pixel = image::Rgba([inv, inv, inv, 255]);
    }
    img_buffer.save(temp_path)?;

    let mut image_base_config = config.base.clone();
    // If bounds are disabled, we must ensure generate_image_gcode doesn't rescale based on pixels.
    // The physical size we want is out_w, and the image has 'width' pixels.
    // So scale should be out_w / width. Since width = out_w * lines_per_mm, scale = 1.0 / lines_per_mm.
    if !image_base_config.bounds.enabled {
        image_base_config.scale = 1.0 / lines_per_mm;
    }

    let result = generate_image_gcode(temp_path, &ImageBurnConfig { 
        base: image_base_config, 
        low_fid: 0.0, 
        high_fid: 1.0, 
        lines_per_mm 
    }, Some(pwr_max), is_preview);
    let _ = std::fs::remove_file(temp_path);
    result
}

struct VectorGCodeBuilder {
    gcode: String,
    offset: Vector2F,
    scale: f32,
    current_pos: Vector2F,
    start_pos: Vector2F,
    power: f32,
    speed: f32,
    curve_steps: u32,
}

impl OutlineSink for VectorGCodeBuilder {
    fn move_to(&mut self, to: Vector2F) {
        let p = (to + self.offset) * self.scale;
        let px = p.x().clamp(0.0, 400.0);
        let py = p.y().clamp(0.0, 400.0);
        self.gcode.push_str(&format!("{}\n{}\n", gcode::CMD_LASER_OFF, gcode::move_xy_f(px, py, 3000.0)));
        self.gcode.push_str(&format!("{}\n", gcode::laser_on_dynamic_f(self.power, self.speed)));
        self.current_pos = to;
        self.start_pos = to;
    }

    fn line_to(&mut self, to: Vector2F) {
        let p = (to + self.offset) * self.scale;
        let px_raw = p.x();
        let py_raw = p.y();
        let out_of_bounds = px_raw < 0.0 || px_raw > 400.0 || py_raw < 0.0 || py_raw > 400.0;
        let power = if out_of_bounds { 0.0 } else { self.power };
        let px = px_raw.clamp(0.0, 400.0);
        let py = py_raw.clamp(0.0, 400.0);
        self.gcode.push_str(&format!("{}\n", gcode::burn_s(px, py, power)));
        self.current_pos = to;
    }

    fn quadratic_curve_to(&mut self, control: Vector2F, to: Vector2F) {
        let segments = self.curve_steps;
        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let p = self.current_pos * (1.0 - t).powi(2) + control * 2.0 * (1.0 - t) * t + to * t.powi(2);
            self.line_to(p);
        }
    }

    fn cubic_curve_to(&mut self, control: LineSegment2F, to: Vector2F) {
        let segments = self.curve_steps;
        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let p = self.current_pos * (1.0 - t).powi(3) + control.from() * 3.0 * (1.0 - t).powi(2) * t + control.to() * 3.0 * (1.0 - t) * t.powi(2) + to * t.powi(3);
            self.line_to(p);
        }
    }

    fn close(&mut self) { self.line_to(self.start_pos); self.gcode.push_str(&format!("{}\n", gcode::CMD_LASER_OFF)); }
}

pub fn generate_outline_gcode(x: f32, y: f32, w: f32, h: f32, speed: f32) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        gcode::CMD_HOME,
        gcode::CMD_ABSOLUTE_POS,
        gcode::CMD_LASER_OFF,
        gcode::move_xy_f(x, y, 3000.0),
        gcode::move_xy_f(x + w, y, speed),
        gcode::move_linear_xy(x + w, y + h),
        gcode::move_linear_xy(x, y + h),
        gcode::move_linear_xy(x, y),
        gcode::CMD_LASER_OFF,
        gcode::CMD_HOME
    )
}

pub fn get_image_outline_gcode(path: &str, config: &ImageBurnConfig) -> Option<String> {
    let img = image::open(path).ok()?;
    let (orig_w, orig_h) = (img.width() as f32, img.height() as f32);
    let mut final_scale = config.base.scale;
    if config.base.bounds.enabled {
        final_scale = (config.base.bounds.w / orig_w).min(config.base.bounds.h / orig_h);
    }
    let out_w = orig_w * final_scale;
    let out_h = orig_h * final_scale;
    let cx = if config.base.bounds.enabled { config.base.bounds.x + config.base.bounds.w / 2.0 } else { 200.0 };
    let cy = if config.base.bounds.enabled { config.base.bounds.y + config.base.bounds.h / 2.0 } else { 200.0 };
    let x = cx - out_w / 2.0;
    let y = cy - out_h / 2.0;
    Some(generate_outline_gcode(x, y, out_w, out_h, config.base.feed_rate))
}

pub fn get_gcode_bounds(gcode: &str) -> Option<(f32, f32, f32, f32)> {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    let (mut curr_x, mut curr_y, mut found) = (0.0, 0.0, false);
    for line in gcode.lines() {
        let line = line.trim().to_uppercase();
        if line.starts_with('G') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let (mut x, mut y) = (None, None);
            for p in parts {
                if p.starts_with('X') { x = p[1..].parse::<f32>().ok(); }
                if p.starts_with('Y') { y = p[1..].parse::<f32>().ok(); }
            }
            if let Some(val) = x { curr_x = val; }
            if let Some(val) = y { curr_y = val; }
            if x.is_some() || y.is_some() {
                min_x = min_x.min(curr_x); max_x = max_x.max(curr_x);
                min_y = min_y.min(curr_y); max_y = max_y.max(curr_y);
                found = true;
            }
        }
    }
    if found { Some((min_x, min_y, max_x - min_x, max_y - min_y)) } else { None }
}

pub fn run_serial_cmd(cmd_str: &str, label: &str, _tx: mpsc::Sender<String>, use_virtual: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if use_virtual { println!("[{}] VIRTUAL: {} -> {}", get_ts(), label, cmd_str); }
    else { println!("[{}] SENDING: {}...", get_ts(), label); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_gcode_generation_not_empty() {
        let config = TextBurnConfig {
            base: BurnConfig { power: 100.0, feed_rate: 1000.0, scale: 1.0, passes: 1, bounds: Bounds { enabled: false, x: 0.0, y: 0.0, w: 400.0, h: 400.0 } },
            content: "Hi".to_string(), font: "Default".to_string(), is_bold: false, is_outline: false, letter_spacing: 0.0, line_spacing: 1.0, curve_steps: 10, lines_per_mm: 5.0,
        };
        let result = generate_text_gcode(&config, true);
        match result { Ok((gcode, _)) => { assert!(!gcode.is_empty()); } Err(e) => panic!("Failed: {}", e) }
    }

    #[test]
    fn test_text_gcode_generation_outline() {
        let config = TextBurnConfig {
            base: BurnConfig { power: 100.0, feed_rate: 1000.0, scale: 1.0, passes: 1, bounds: Bounds { enabled: false, x: 0.0, y: 0.0, w: 400.0, h: 400.0 } },
            content: "Hi".to_string(), font: "Default".to_string(), is_bold: false, is_outline: true, letter_spacing: 0.0, line_spacing: 1.0, curve_steps: 10, lines_per_mm: 5.0,
        };
        let result = generate_text_gcode(&config, true);
        match result { Ok((gcode, _)) => { assert!(gcode.contains("G1")); } Err(e) => panic!("Failed: {}", e) }
    }

    #[test]
    fn test_embedded_shapes_generation() {
        let shapes = vec!["star", "stars8", "stars9", "car", "square", "heart"];
        for shape in shapes {
            let config = BurnConfig { power: 100.0, feed_rate: 1000.0, scale: 1.0, passes: 1, bounds: Bounds { enabled: false, x: 0.0, y: 0.0, w: 400.0, h: 400.0 } };
            let result = generate_pattern_gcode(shape, &config, true);
            match result { Ok((gcode, _)) => { assert!(!gcode.is_empty()); } Err(e) => panic!("Failed: {}", e) }
        }
    }

    #[test]
    fn test_image_gcode_performance() {
        let path = "assets/test.jpg";
        if !std::path::Path::new(path).exists() { return; }
        let config = ImageBurnConfig {
            base: BurnConfig { power: 100.0, feed_rate: 1000.0, scale: 0.5, passes: 1, bounds: Bounds { enabled: false, x: 0.0, y: 0.0, w: 400.0, h: 400.0 } },
            low_fid: 0.0, high_fid: 1.0, lines_per_mm: 5.0,
        };
        let start = std::time::Instant::now();
        let result = generate_image_gcode(path, &config, None, true);
        println!("Image GCode performance: {:?}", start.elapsed());
        match result { Ok((gcode, _)) => { println!("Length: {}", gcode.len()); } Err(e) => panic!("Failed: {}", e) }
    }

    #[test]
    fn test_image_gcode_100x50() {
        let path = "assets/test.jpg";
        if !std::path::Path::new(path).exists() { return; }
        let config = ImageBurnConfig {
            base: BurnConfig {
                power: 100.0,
                feed_rate: 1000.0,
                scale: 1.0,
                passes: 1,
                bounds: Bounds {
                    enabled: true,
                    x: 150.0,
                    y: 175.0,
                    w: 100.0,
                    h: 50.0,
                },
            },
            low_fid: 0.0,
            high_fid: 1.0,
            lines_per_mm: 5.0,
        };

        let start = std::time::Instant::now();
        let result = generate_image_gcode(path, &config, None, true);
        println!("100x50 Test Duration: {:?}", start.elapsed());
        match result {
            Ok((gcode, _)) => {
                println!("100x50 GCode Length: {}", gcode.len());
            }
            Err(e) => panic!("Failed: {}", e),
        }
    }
}
