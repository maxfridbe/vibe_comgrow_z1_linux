fn run_cli_mode(target_label: &str, sections: &[Section]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    }).expect("Error setting Ctrl-C handler");

    let cmd_str_owned: String;
    let cmd_str = if let Some(c) = sections.iter()
        .flat_map(|s| &s.commands)
        .find(|c| c.label.to_lowercase() == target_label.to_lowercase())
        .map(|c| c.cmd) 
    {
        c
    } else {
        cmd_str_owned = target_label.to_string();
        &cmd_str_owned
    };

    run_serial_cmd(cmd_str, target_label, tx)
}

fn run_dynamic_pattern_cli(args: &[OsString]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut pico_args = pico_args::Arguments::from_vec(args.to_vec());
    let shape: String = pico_args.free_from_str()?;
    let pwr_pct: String = pico_args.value_from_str("--power").unwrap_or_else(|_| "1%".to_string());
    let speed_pct: String = pico_args.value_from_str("--speed").unwrap_or_else(|_| "100%".to_string());
    let scale_str: String = pico_args.value_from_str("--scale").unwrap_or_else(|_| "1x".to_string());
    let passes_str: String = pico_args.value_from_str("--passes").unwrap_or_else(|_| "1".to_string());

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
    }).expect("Error setting Ctrl-C handler");

    let (cmd, label) = generate_pattern_gcode(&shape, &pwr_pct, &speed_pct, &scale_str, &passes_str)?;
    run_serial_cmd(&cmd, &label, tx)
}

fn generate_pattern_gcode(shape: &str, pwr_pct: &str, speed_pct: &str, scale_str: &str, passes_str: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr = pwr_pct.trim_end_matches('%').parse::<f32>().unwrap_or(1.0).clamp(0.0, 100.0);
    let spd = speed_pct.trim_end_matches('%').parse::<f32>().unwrap_or(100.0).clamp(1.0, 1000.0);
    let scale = scale_str.trim_end_matches('x').parse::<f32>().unwrap_or(1.0).max(0.1);
    let passes = passes_str.parse::<u32>().unwrap_or(1).clamp(1, 100);
    let s_val = (pwr * 10.0) as i32;
    let f_val = (spd * 10.0) as i32;
    let offset_x = 50.0;
    let offset_y = 50.0;
    let bed_size = 400.0;

    let (path_gcode, max_x, max_y) = match shape.to_lowercase().as_str() {
        "square" => {
            let size = 50.0 * scale;
            let x2 = offset_x + size;
            let y2 = offset_y + size;
            (format!("G1 X{:.2}\nG1 Y{:.2}\nG1 X{:.2}\nG1 Y{:.2}\n", x2, y2, offset_x, offset_y), x2, y2)
        },
        "heart" => {
            let w = 50.0 * scale;
            let h = 37.5 * scale;
            let r = 12.5 * scale;
            let start_x = offset_x + (w/2.0);
            let x_right = offset_x + w;
            let y_mid = offset_y + (h * 0.66);
            (format!("G1 X{:.2} Y{:.2}\nG3 X{:.2} Y{:.2} R{:.2}\nG3 X{:.2} Y{:.2} R{:.2}\nG1 X{:.2} Y{:.2}\n", 
                x_right, y_mid, start_x, y_mid, r, offset_x, y_mid, r, start_x, offset_y), x_right, y_mid + r)
        },
        "star" => {
            let cx = 100.0;
            let cy = 100.0;
            let pts = [
                (8.4, 11.2), (33.6, 11.2), (13.3, -4.2), (20.3, -33.6),
                (0.0, -16.8), (-20.3, -33.6), (-13.3, -4.2), (-33.6, 11.2), (-8.4, 11.2), (0.0, 35.0)
            ];
            let mut gcode = String::new();
            let mut mx = cx;
            let mut my = cy;
            for (dx, dy) in pts {
                let px = cx + (dx * scale);
                let py = cy + (dy * scale);
                gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", px, py));
                if px > mx { mx = px; }
                if py > my { my = py; }
            }
            (gcode, mx, my)
        },
        other => {
            let path = format!("assets/{}.svg", other);
            if std::path::Path::new(&path).exists() {
                svg_helper::load_svg_as_gcode(&path, scale, offset_x, offset_y)?
            } else {
                return Err(format!("Unknown shape '{}'. Try Square, Heart, Star, or a file in assets/.", shape).into());
            }
        }
    };

    if max_x > bed_size || max_y > bed_size {
        return Err(format!("Scale {:.1}x is too large! Shape would reach ({:.1}, {:.1}) which exceeds the {:.1}mm bed limit.", scale, max_x, max_y, bed_size).into());
    }

    let mut final_gcode = String::new();
    final_gcode.push_str("G90\n"); 
    final_gcode.push_str(&format!("M4 S{} F{}\n", s_val, f_val));
    for _ in 0..passes {
        final_gcode.push_str(&path_gcode);
    }
    final_gcode.push_str("M5\n$H");

    Ok((final_gcode, format!("Dynamic {} (Scale: {}x, Passes: {}, Power: {}%, Speed: {}%)", shape, scale, passes, pwr, spd)))
}

fn run_serial_cmd(cmd_str: &str, label: &str, tx: mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    let port_name = "/dev/ttyUSB0";
    let baud_rate = 115200;

    println!("[{}] CLI: Mode = {}", get_ts(), label);

    let mut port = serialport::new(port_name, baud_rate)
        .timeout(std::time::Duration::from_millis(100))
        .open()?;

    println!("[{}] SERIAL: Connected to {}", get_ts(), port_name);

    for line in cmd_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        
        let full_cmd = if trimmed == "0x18" { "\x18".to_string() } else { format!("{}\n", trimmed) };
        let explanation = decode_gcode(trimmed);
        println!("[{}] SEND: {:?} | Interpreter: {}", get_ts(), trimmed, explanation);
        port.write_all(full_cmd.as_bytes())?;

        let mut serial_buf: Vec<u8> = vec![0; 1024];
        let mut accumulator = String::new();
        let start_time = std::time::Instant::now();
        let mut finished = false;

        while start_time.elapsed().as_secs() < 30 {
            if let Ok(t) = port.read(serial_buf.as_mut_slice()) {
                if t > 0 {
                    accumulator.push_str(&String::from_utf8_lossy(&serial_buf[..t]));
                    while let Some(pos) = accumulator.find('\n') {
                        let res_line = accumulator[..pos].trim().to_string();
                        accumulator.drain(..=pos);
                        if !res_line.is_empty() {
                            let explanation = decode_response(&res_line);
                            println!("[{}] RECV: {:?} | Interpreter: {}", get_ts(), res_line, explanation);
                            if res_line == "ok" || res_line.starts_with("error") {
                                finished = true;
                                break;
                            }
                        }
                    }
                }
            }
            if finished { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    Ok(())
}

fn clay_scope_id(id: &str) -> clay_layout::id::Id {
    unsafe { 
        clay_layout::id::Id { id: clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from(id), 0, 0) }
    }
}
