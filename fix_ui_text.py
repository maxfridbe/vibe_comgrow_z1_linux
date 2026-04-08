import re

with open("src/ui_text.rs", "r") as f:
    content = f.read()

# Replace block 1 (lines 579-646 approx)
old_block1 = """                            let state_data = (
                                g.text_content.clone(),
                                g.power,
                                g.feed_rate,
                                g.scale,
                                g.passes,
                                g.boundary_enabled,
                                g.boundary_x,
                                g.boundary_y,
                                g.boundary_w,
                                g.boundary_h,
                                g.text_is_bold,
                                g.text_is_outline,
                                g.text_letter_spacing,
                                g.text_line_spacing,
                                g.text_curve_steps,
                                g.text_lines_per_mm,
                                g.text_font.clone(),
                            );
                            let state_clone = Arc::clone(state);
                            std::thread::spawn(move || {
                                let (
                                    txt,
                                    pwr,
                                    spd,
                                    scl,
                                    pas,
                                    b_enabled,
                                    bx,
                                    by,
                                    bw,
                                    bh,
                                    bold,
                                    outline,
                                    l_space,
                                    line_space,
                                    c_steps,
                                    l_per_mm,
                                    f_name,
                                ) = state_data;
                                let fit = if b_enabled {
                                    Some((bw, bh))
                                } else {
                                    None
                                };
                                let center = if b_enabled {
                                    (bx + bw / 2.0, by + bh / 2.0)
                                } else {
                                    (200.0, 200.0)
                                };

                                if let Ok((gcode, _)) = generate_text_gcode(
                                    &txt,
                                    pwr,
                                    spd * 10.0,
                                    scl,
                                    pas,
                                    fit,
                                    center,
                                    bold,
                                    outline,
                                    l_space,
                                    line_space,
                                    c_steps,
                                    l_per_mm,
                                    &f_name,
                                    true,
                                ) {"""

new_block1 = """                            let config = g.get_burn_config();
                            let txt = g.text_content.clone();
                            let f_name = g.text_font.clone();
                            let state_clone = Arc::clone(state);
                            std::thread::spawn(move || {
                                let fit = if config.boundary_enabled {
                                    Some((config.boundary_w, config.boundary_h))
                                } else {
                                    None
                                };
                                let center = if config.boundary_enabled {
                                    (config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                                } else {
                                    (200.0, 200.0)
                                };

                                if let Ok((gcode, _)) = generate_text_gcode(
                                    &txt,
                                    config.power,
                                    config.feed_rate * 10.0,
                                    config.scale,
                                    config.passes,
                                    fit,
                                    center,
                                    config.text_is_bold,
                                    config.text_is_outline,
                                    config.text_letter_spacing,
                                    config.text_line_spacing,
                                    config.text_curve_steps,
                                    config.text_lines_per_mm,
                                    &f_name,
                                    true,
                                ) {"""

content = content.replace(old_block1, new_block1)

old_block2 = """                        let state_data = {
                            let mut g = state.lock().unwrap();
                            g.is_processing = true;
                            (
                                g.text_content.clone(),
                                g.power,
                                g.feed_rate,
                                g.scale,
                                g.passes,
                                g.boundary_enabled,
                                g.boundary_x,
                                g.boundary_y,
                                g.boundary_w,
                                g.boundary_h,
                                g.text_is_bold,
                                g.text_is_outline,
                                g.text_letter_spacing,
                                g.text_line_spacing,
                                g.text_curve_steps,
                                g.text_lines_per_mm,
                                g.text_font.clone(),
                            )
                        };
                        let state_clone = Arc::clone(state);
                        std::thread::spawn(move || {
                            let (
                                txt,
                                pwr,
                                spd,
                                scl,
                                pas,
                                b_enabled,
                                bx,
                                by,
                                bw,
                                bh,
                                bold,
                                outline,
                                l_space,
                                line_space,
                                c_steps,
                                l_per_mm,
                                f_name,
                            ) = state_data;
                            let fit = if b_enabled {
                                Some((bw, bh))
                            } else {
                                None
                            };
                            let center = if b_enabled {
                                (bx + bw / 2.0, by + bh / 2.0)
                            } else {
                                (200.0, 200.0)
                            };

                            if let Ok((gcode, _)) = generate_text_gcode(
                                &txt, pwr, spd, scl, pas, fit, center, bold, outline, l_space, line_space, c_steps, l_per_mm,
                                &f_name, false,
                            ) {"""

new_block2 = """                        let (config, txt, f_name) = {
                            let mut g = state.lock().unwrap();
                            g.is_processing = true;
                            (g.get_burn_config(), g.text_content.clone(), g.text_font.clone())
                        };
                        let state_clone = Arc::clone(state);
                        std::thread::spawn(move || {
                            let fit = if config.boundary_enabled {
                                Some((config.boundary_w, config.boundary_h))
                            } else {
                                None
                            };
                            let center = if config.boundary_enabled {
                                (config.boundary_x + config.boundary_w / 2.0, config.boundary_y + config.boundary_h / 2.0)
                            } else {
                                (200.0, 200.0)
                            };

                            if let Ok((gcode, _)) = generate_text_gcode(
                                &txt, config.power, config.feed_rate, config.scale, config.passes, fit, center, config.text_is_bold, config.text_is_outline, config.text_letter_spacing, config.text_line_spacing, config.text_curve_steps, config.text_lines_per_mm,
                                &f_name, false,
                            ) {"""

content = content.replace(old_block2, new_block2)

with open("src/ui_text.rs", "w") as f:
    f.write(content)

