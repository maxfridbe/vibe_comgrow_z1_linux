import sys
import re

def main():
    with open('src/cli_and_helpers.rs', 'r') as f:
        content = f.read()

    # Imports
    content = content.replace('use crate::svg_helper;', 'use crate::svg_helper;\nuse crate::state::{BurnConfig, ImageBurnConfig, TextBurnConfig};')

    # run_dynamic_pattern_cli
    old_run = """    let center: String = format!("{},{}", cx, cy);

    let (gcode, _) = generate_pattern_gcode(&shape, &pwr, &spd, &scl, "1", None, &center)?;"""
    new_run = """    let config = BurnConfig {
        power: pwr.trim_end_matches('%').parse::<f32>()? * 10.0,
        feed_rate: spd.trim_end_matches('%').parse::<f32>()? * 10.0,
        scale: scl.trim_end_matches('x').parse::<f32>()?,
        passes: 1,
        boundary_enabled: true,
        boundary_x: cx - 200.0,
        boundary_y: cy - 200.0,
        boundary_w: 400.0,
        boundary_h: 400.0,
    };

    let (gcode, _) = generate_pattern_gcode(&shape, &config, false)?;"""
    content = content.replace(old_run, new_run)

    # generate_pattern_gcode
    old_pat_sig = """pub fn generate_pattern_gcode(
    shape: &str,
    pwr: &str,
    spd: &str,
    scale: &str,
    passes: &str,
    _fit: Option<String>,
    center: &str,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr_val = pwr.trim_end_matches('%').parse::<f32>()?;
    let spd_val = spd.trim_end_matches('%').parse::<f32>()?;
    let scl_val = scale.trim_end_matches('x').parse::<f32>()?;
    let pas_val = passes.parse::<u32>().unwrap_or(1);
    let (cx, cy) = parse_pair(center)?;"""

    new_pat_sig = """pub fn generate_pattern_gcode(
    shape: &str,
    config: &BurnConfig,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr_val = config.power / 10.0;
    let spd_val = if is_preview { config.feed_rate.min(1000.0) / 10.0 } else { config.feed_rate / 10.0 };
    let scl_val = config.scale;
    let pas_val = config.passes;
    let (cx, cy) = if config.boundary_enabled {
        (config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
    } else {
        (200.0, 200.0)
    };
    let center = format!("{},{}", cx, cy);
    let parsed_fit = if config.boundary_enabled {
        Some((config.boundary_w, config.boundary_h))
    } else {
        None
    };"""
    content = content.replace(old_pat_sig, new_pat_sig)

    # Remove old parsed_fit logic inside generate_pattern_gcode
    old_fit_1 = """                let mut parsed_fit = None;
                if let Some(ref f) = _fit {
                    if let Ok(pair) = parse_pair(f) {
                        parsed_fit = Some(pair);
                    }
                }"""
    content = content.replace(old_fit_1, "")
    
    old_fit_2 = """                    let mut parsed_fit = None;
                    if let Some(ref f) = _fit {
                        if let Ok(pair) = parse_pair(f) {
                            parsed_fit = Some(pair);
                        }
                    }"""
    content = content.replace(old_fit_2, "")

    # generate_image_gcode
    old_img_sig = """pub fn generate_image_gcode(
    path: &str,
    pwr_max: f32,
    speed: f32,
    scale: f32,
    passes: u32,
    fit: Option<(f32, f32)>,
    center: (f32, f32),
    low_fid: f32,
    high_fid: f32,
    lines_per_mm: f32,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {"""

    new_img_sig = """pub fn generate_image_gcode(
    path: &str,
    config: &ImageBurnConfig,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr_max = config.base.power;
    let speed = if is_preview { config.base.feed_rate * 10.0 } else { config.base.feed_rate };
    let scale = config.base.scale;
    let passes = config.base.passes;
    let low_fid = config.low_fid;
    let high_fid = config.high_fid;
    let lines_per_mm = config.lines_per_mm;
    let fit = if config.base.boundary_enabled {
        Some((config.base.boundary_w, config.base.boundary_h))
    } else {
        None
    };
    let center = if config.base.boundary_enabled {
        (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
    } else {
        (200.0, 200.0)
    };"""
    content = content.replace(old_img_sig, new_img_sig)

    # generate_text_gcode
    old_txt_sig = """pub fn generate_text_gcode(
    text: &str,
    pwr_max: f32,
    speed: f32,
    scale: f32,
    passes: u32,
    fit: Option<(f32, f32)>,
    center: (f32, f32),
    bold: bool,
    outline: bool,
    letter_spacing: f32,
    _line_spacing: f32,
    curve_steps: u32,
    lines_per_mm: f32,
    font_family: &str,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {"""

    new_txt_sig = """pub fn generate_text_gcode(
    config: &TextBurnConfig,
    is_preview: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let text = &config.content;
    let pwr_max = config.base.power;
    let speed = config.base.feed_rate;
    let scale = config.base.scale;
    let passes = config.base.passes;
    let bold = config.is_bold;
    let outline = config.is_outline;
    let letter_spacing = config.letter_spacing;
    let _line_spacing = config.line_spacing;
    let curve_steps = config.curve_steps;
    let lines_per_mm = config.lines_per_mm;
    let font_family = &config.font;

    let fit = if config.base.boundary_enabled {
        Some((config.base.boundary_w, config.base.boundary_h))
    } else {
        None
    };
    let center = if config.base.boundary_enabled {
        (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
    } else {
        (200.0, 200.0)
    };"""
    content = content.replace(old_txt_sig, new_txt_sig)

    old_txt_img_call = """    let result =
        generate_image_gcode(temp_path, pwr_max, speed, final_user_scale, passes, None, center, 0.0, 1.0, lines_per_mm, is_preview);"""
    new_txt_img_call = """    let img_config = ImageBurnConfig {
        base: BurnConfig {
            power: config.base.power,
            feed_rate: config.base.feed_rate,
            scale: config.base.scale,
            passes: config.base.passes,
            boundary_enabled: config.base.boundary_enabled,
            boundary_x: config.base.boundary_x,
            boundary_y: config.base.boundary_y,
            boundary_w: config.base.boundary_w,
            boundary_h: config.base.boundary_h,
        },
        low_fid: 0.0,
        high_fid: 1.0,
        lines_per_mm: config.lines_per_mm,
    };
    let result = generate_image_gcode(temp_path, &img_config, is_preview);"""
    content = content.replace(old_txt_img_call, new_txt_img_call)
    
    # Tests in cli_and_helpers.rs
    test_old = """    #[test]
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

        let result = generate_text_gcode(
            text,
            pwr,
            spd,
            scl,
            passes,
            fit,
            center,
            bold,
            _outline,
            letter_spacing,
            _line_spacing,
            10,
            5.0,
            font_family,
            true,
        );"""
    test_new = """    #[test]
    fn test_text_gcode_generation_not_empty() {
        let config = TextBurnConfig {
            base: BurnConfig {
                power: 100.0,
                feed_rate: 1000.0,
                scale: 1.0,
                passes: 1,
                boundary_enabled: false,
                boundary_x: 0.0,
                boundary_y: 0.0,
                boundary_w: 400.0,
                boundary_h: 400.0,
            },
            content: "Hi".to_string(),
            font: "Default".to_string(),
            is_bold: false,
            is_outline: false,
            letter_spacing: 0.0,
            line_spacing: 1.0,
            curve_steps: 10,
            lines_per_mm: 5.0,
        };

        let result = generate_text_gcode(&config, true);"""
    content = content.replace(test_old, test_new)

    test_old_2 = """    #[test]
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

        let result = generate_text_gcode(
            text,
            pwr,
            spd,
            scl,
            passes,
            fit,
            center,
            bold,
            outline,
            letter_spacing,
            _line_spacing,
            10,
            5.0,
            font_family,
            true,
        );"""
    test_new_2 = """    #[test]
    fn test_text_gcode_generation_outline() {
        let config = TextBurnConfig {
            base: BurnConfig {
                power: 100.0,
                feed_rate: 1000.0,
                scale: 1.0,
                passes: 1,
                boundary_enabled: false,
                boundary_x: 0.0,
                boundary_y: 0.0,
                boundary_w: 400.0,
                boundary_h: 400.0,
            },
            content: "Hi".to_string(),
            font: "Default".to_string(),
            is_bold: false,
            is_outline: true,
            letter_spacing: 0.0,
            line_spacing: 1.0,
            curve_steps: 10,
            lines_per_mm: 5.0,
        };

        let result = generate_text_gcode(&config, true);"""
    content = content.replace(test_old_2, test_new_2)

    test_old_3 = """        for shape in shapes {
            let result = generate_pattern_gcode(shape, "10%", "1000%", "1.0x", "1", None, "200,200");"""
    test_new_3 = """        for shape in shapes {
            let config = BurnConfig {
                power: 100.0,
                feed_rate: 10000.0,
                scale: 1.0,
                passes: 1,
                boundary_enabled: false,
                boundary_x: 0.0,
                boundary_y: 0.0,
                boundary_w: 400.0,
                boundary_h: 400.0,
            };
            let result = generate_pattern_gcode(shape, &config, false);"""
    content = content.replace(test_old_3, test_new_3)


    with open('src/cli_and_helpers.rs', 'w') as f:
        f.write(content)

    # UI FILES
    # src/ui_image.rs
    with open('src/ui_image.rs', 'r') as f:
        content = f.read()

    old_img_preview = """                                let fit = if config.base.boundary_enabled {
                                    Some((config.base.boundary_w, config.base.boundary_h))
                                } else {
                                    None
                                };
                                let center = if config.base.boundary_enabled {
                                    (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                                } else {
                                    (200.0, 200.0)
                                };
                                let state_clone = Arc::clone(state);
                                let path_clone = p.clone();
                                std::thread::spawn(move || {
                                    if let Ok((gcode, _)) = generate_image_gcode(
                                        &path_clone,
                                        config.base.power,
                                        config.base.feed_rate * 10.0,
                                        config.base.scale,
                                        config.base.passes,
                                        fit,
                                        center,
                                        config.low_fid,
                                        config.high_fid,
                                        config.lines_per_mm,
                                        true,
                                    ) {"""
    new_img_preview = """                                let state_clone = Arc::clone(state);
                                let path_clone = p.clone();
                                std::thread::spawn(move || {
                                    if let Ok((gcode, _)) = generate_image_gcode(
                                        &path_clone,
                                        &config,
                                        true,
                                    ) {"""
    content = content.replace(old_img_preview, new_img_preview)

    old_img_burn = """                        let fit = if config.base.boundary_enabled {
                            Some((config.base.boundary_w, config.base.boundary_h))
                        } else {
                            None
                        };
                        let center = if config.base.boundary_enabled {
                            (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                        } else {
                            (200.0, 200.0)
                        };
                        let state_clone = Arc::clone(state);
                        let path_clone = p.clone();
                        std::thread::spawn(move || {
                            if let Ok((gcode, _)) =
                                generate_image_gcode(&path_clone, config.base.power, config.base.feed_rate, config.base.scale, config.base.passes, fit, center, config.low_fid, config.high_fid, config.lines_per_mm, false)
                            {"""
    new_img_burn = """                        let state_clone = Arc::clone(state);
                        let path_clone = p.clone();
                        std::thread::spawn(move || {
                            if let Ok((gcode, _)) =
                                generate_image_gcode(&path_clone, &config, false)
                            {"""
    content = content.replace(old_img_burn, new_img_burn)

    old_img_outline = """                            let fit = if config.base.boundary_enabled {
                                Some((config.base.boundary_w, config.base.boundary_h))
                            } else {
                                None
                            };
                            let center = if config.base.boundary_enabled {
                                (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                            } else {
                                (200.0, 200.0)
                            };
                            generate_image_gcode(
                                &path_clone,
                                config.base.power,
                                config.base.feed_rate,
                                config.base.scale,
                                config.base.passes,
                                fit,
                                center,
                                config.low_fid,
                                config.high_fid,
                                config.lines_per_mm,
                                false,
                            )"""
    new_img_outline = """                            generate_image_gcode(
                                &path_clone,
                                &config,
                                false,
                            )"""
    content = content.replace(old_img_outline, new_img_outline)

    with open('src/ui_image.rs', 'w') as f:
        f.write(content)

    # src/ui_text.rs
    with open('src/ui_text.rs', 'r') as f:
        content = f.read()

    old_txt_preview = """                                let fit = if config.base.boundary_enabled {
                                    Some((config.base.boundary_w, config.base.boundary_h))
                                } else {
                                    None
                                };
                                let center = if config.base.boundary_enabled {
                                    (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                                } else {
                                    (200.0, 200.0)
                                };

                                if let Ok((gcode, _)) = generate_text_gcode(
                                    &config.content,
                                    config.base.power,
                                    config.base.feed_rate * 10.0,
                                    config.base.scale,
                                    config.base.passes,
                                    fit,
                                    center,
                                    config.is_bold,
                                    config.is_outline,
                                    config.letter_spacing,
                                    config.line_spacing,
                                    config.curve_steps,
                                    config.lines_per_mm,
                                    &config.font,
                                    true,
                                ) {"""
    new_txt_preview = """                                if let Ok((gcode, _)) = generate_text_gcode(
                                    &config,
                                    true,
                                ) {"""
    content = content.replace(old_txt_preview, new_txt_preview)

    old_txt_burn = """                        let fit = if config.base.boundary_enabled {
                            Some((config.base.boundary_w, config.base.boundary_h))
                        } else {
                            None
                        };
                        let center = if config.base.boundary_enabled {
                            (config.base.boundary_x + config.base.boundary_w / 2.0, config.base.boundary_y + config.base.boundary_h / 2.0)
                        } else {
                            (200.0, 200.0)
                        };

                        if let Ok((gcode, _)) = generate_text_gcode(
                            &config.content, config.base.power, config.base.feed_rate, config.base.scale, config.base.passes, fit, center, config.is_bold, config.is_outline, config.letter_spacing, config.line_spacing, config.curve_steps, config.lines_per_mm,
                            &config.font, false,
                        ) {"""
    new_txt_burn = """                        if let Ok((gcode, _)) = generate_text_gcode(
                            &config, false,
                        ) {"""
    content = content.replace(old_txt_burn, new_txt_burn)

    old_txt_outline = """                    || {
                        let g = state.lock().unwrap();
                        let fit = if g.boundary_enabled {
                            Some((g.boundary_w, g.boundary_h))
                        } else {
                            None
                        };
                        let center = if g.boundary_enabled {
                            (g.boundary_x + g.boundary_w / 2.0, g.boundary_y + g.boundary_h / 2.0)
                        } else {
                            (200.0, 200.0)
                        };
                        generate_text_gcode(
                            &g.text_content,
                            g.power,
                            g.feed_rate,
                            g.scale,
                            g.passes,
                            fit,
                            center,
                            g.text_is_bold,
                            g.text_is_outline,
                            g.text_letter_spacing,
                            g.text_line_spacing,
                            g.text_curve_steps,
                            g.text_lines_per_mm,
                            &g.text_font,
                            false,
                        )"""
    new_txt_outline = """                    || {
                        let g = state.lock().unwrap();
                        let config = g.get_text_burn_config();
                        generate_text_gcode(
                            &config,
                            false,
                        )"""
    content = content.replace(old_txt_outline, new_txt_outline)

    with open('src/ui_text.rs', 'w') as f:
        f.write(content)

    # src/ui_test.rs
    with open('src/ui_test.rs', 'r') as f:
        content = f.read()

    old_test_svg_burn = """                            let fit = if config.boundary_enabled {
                                Some(format!("{}x{}", config.boundary_w, config.boundary_h))
                            } else {
                                None
                            };
                            let center = if config.boundary_enabled {
                                format!("{},{}", config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                            } else {
                                "200,200".to_string()
                            };

                            let result: Result<(String, String), Box<dyn std::error::Error + Send + Sync>> =
                                generate_pattern_gcode(
                                    &p,
                                    &format!("{}%", config.power / 10.0),
                                    &format!("{}%", config.feed_rate / 10.0),
                                    &format!("{}x", config.scale),
                                    &config.passes.to_string(),
                                    fit,
                                    &center,
                                );"""
    new_test_svg_burn = """                            let result: Result<(String, String), Box<dyn std::error::Error + Send + Sync>> =
                                generate_pattern_gcode(
                                    &p,
                                    &config,
                                    false,
                                );"""
    content = content.replace(old_test_svg_burn, new_test_svg_burn)

    old_test_svg_outline = """                                let fit = if config.boundary_enabled {
                                    Some(format!("{}x{}", config.boundary_w, config.boundary_h))
                                } else {
                                    None
                                };
                                let center = if config.boundary_enabled {
                                    format!("{},{}", config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                                } else {
                                    "200,200".to_string()
                                };
                                generate_pattern_gcode(
                                    &path_clone,
                                    &format!("{}%", config.power / 10.0),
                                    &format!("{}%", config.feed_rate / 10.0),
                                    &format!("{}x", config.scale),
                                    &config.passes.to_string(),
                                    fit,
                                    &center,
                                )"""
    new_test_svg_outline = """                                generate_pattern_gcode(
                                    &path_clone,
                                    &config,
                                    false,
                                )"""
    content = content.replace(old_test_svg_outline, new_test_svg_outline)

    old_test_svg_preview = """                                    let fit = if config.boundary_enabled {
                                        Some(format!("{}x{}", config.boundary_w, config.boundary_h))
                                    } else {
                                        None
                                    };
                                    let center = if config.boundary_enabled {
                                        format!("{},{}", config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                                    } else {
                                        "200,200".to_string()
                                    };
                                    let preview_spd = config.feed_rate.min(1000.0);
                                    let state_clone = Arc::clone(state);
                                    let path_clone = p.clone();

                                    std::thread::spawn(move || {
                                        let result: Result<(String, String), Box<dyn std::error::Error + Send + Sync>> =
                                            generate_pattern_gcode(
                                                &path_clone,
                                                &format!("{}%", config.power / 10.0),
                                                &format!("{}%", preview_spd),
                                                &format!("{}x", config.scale),
                                                &config.passes.to_string(),
                                                fit,
                                                &center,
                                            );"""
    new_test_svg_preview = """                                    let state_clone = Arc::clone(state);
                                    let path_clone = p.clone();

                                    std::thread::spawn(move || {
                                        let result: Result<(String, String), Box<dyn std::error::Error + Send + Sync>> =
                                            generate_pattern_gcode(
                                                &path_clone,
                                                &config,
                                                true,
                                            );"""
    content = content.replace(old_test_svg_preview, new_test_svg_preview)

    old_test_burn = """                                        let fit = if config.boundary_enabled {
                                            Some(format!("{}x{}", config.boundary_w, config.boundary_h))
                                        } else {
                                            None
                                        };
                                        let center = if config.boundary_enabled {
                                            format!("{},{}", config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                                        } else {
                                            "200,200".to_string()
                                        };

                                        match generate_pattern_gcode(
                                            cmd.label,
                                            &format!("{}%", config.power / 10.0),
                                            &format!("{}%", config.feed_rate / 10.0),
                                            &format!("{}x", config.scale),
                                            &config.passes.to_string(),
                                            fit,
                                            &center,
                                        ) {"""
    new_test_burn = """                                        match generate_pattern_gcode(
                                            cmd.label,
                                            &config,
                                            false,
                                        ) {"""
    content = content.replace(old_test_burn, new_test_burn)

    old_test_outline = """                                            let fit = if config.boundary_enabled {
                                                Some(format!("{}x{}", config.boundary_w, config.boundary_h))
                                            } else {
                                                None
                                            };
                                            let center = if config.boundary_enabled {
                                                format!("{},{}", config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                                            } else {
                                                "200,200".to_string()
                                            };
                                            generate_pattern_gcode(
                                                &label_clone,
                                                &format!("{}%", config.power / 10.0),
                                                &format!("{}%", config.feed_rate / 10.0),
                                                &format!("{}x", config.scale),
                                                &config.passes.to_string(),
                                                fit,
                                                &center,
                                            )"""
    new_test_outline = """                                            generate_pattern_gcode(
                                                &label_clone,
                                                &config,
                                                false,
                                            )"""
    content = content.replace(old_test_outline, new_test_outline)

    old_test_preview = """                                                let fit = if config.boundary_enabled {
                                                    Some(format!("{}x{}", config.boundary_w, config.boundary_h))
                                                } else {
                                                    None
                                                };
                                                let center = if config.boundary_enabled {
                                                    format!("{},{}", config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                                                } else {
                                                    "200,200".to_string()
                                                };

                                                // 10x speed for preview
                                                let preview_spd = (config.feed_rate / 10.0 * 10.0).min(1000.0); // Equivalent to feed_rate.min(1000.0)
                                                let preview_spd = config.feed_rate.min(1000.0);
                                                let label_clone = cmd.label.to_string();
                                                let state_clone = Arc::clone(state);

                                                std::thread::spawn(move || {
                                                    if let Ok((gcode, _)) = generate_pattern_gcode(
                                                        &label_clone,
                                                        &format!("{}%", config.power / 10.0),
                                                        &format!("{}%", preview_spd),
                                                        &format!("{}x", config.scale),
                                                        &config.passes.to_string(),
                                                        fit,
                                                        &center,
                                                    ) {"""
    new_test_preview = """                                                let label_clone = cmd.label.to_string();
                                                let state_clone = Arc::clone(state);

                                                std::thread::spawn(move || {
                                                    if let Ok((gcode, _)) = generate_pattern_gcode(
                                                        &label_clone,
                                                        &config,
                                                        true,
                                                    ) {"""
    content = content.replace(old_test_preview, new_test_preview)

    with open('src/ui_test.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
