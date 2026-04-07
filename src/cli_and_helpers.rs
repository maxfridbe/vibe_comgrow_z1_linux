use std::sync::mpsc;
use std::ffi::OsString;
use crate::ui::Section;
use crate::svg_helper;
use crate::virtual_device::VirtualDevice;

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

pub fn run_cli_mode(target_label: &str, sections: &[Section]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    run_serial_cmd(cmd_str, target_label, tx, false)
}

pub fn run_dynamic_pattern_cli(args: &[OsString]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut pico_args = pico_args::Arguments::from_vec(args.to_vec());
    let use_virtual = pico_args.contains("--virtual");
    let shape: String = pico_args.free_from_str()?;
    let pwr_pct: String = pico_args.value_from_str("--power").unwrap_or_else(|_| "1%".to_string());
    let speed_pct: String = pico_args.value_from_str("--speed").unwrap_or_else(|_| "100%".to_string());
    let scale_str: String = pico_args.value_from_str("--scale").unwrap_or_else(|_| "1x".to_string());
    let passes_str: String = pico_args.value_from_str("--passes").unwrap_or_else(|_| "1".to_string());
    let fit_str: Option<String> = pico_args.opt_value_from_str("--fit")?;
    let center_str: String = pico_args.value_from_str("--center").unwrap_or_else(|_| "200,200".to_string());

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

    let (cmd, label) = generate_pattern_gcode(&shape, &pwr_pct, &speed_pct, &scale_str, &passes_str, fit_str, &center_str)?;
    run_serial_cmd(&cmd, &label, tx, use_virtual)
}

pub fn generate_pattern_gcode(shape: &str, pwr_pct: &str, speed_pct: &str, scale_str: &str, passes_str: &str, fit_str: Option<String>, center_str: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let pwr = pwr_pct.trim_end_matches('%').parse::<f32>().unwrap_or(1.0).clamp(0.0, 100.0);
    let spd = speed_pct.trim_end_matches('%').parse::<f32>().unwrap_or(100.0).clamp(1.0, 1000.0);
    let mut scale = scale_str.trim_end_matches('x').parse::<f32>().unwrap_or(1.0).max(0.01);
    let passes = passes_str.parse::<u32>().unwrap_or(1).clamp(1, 100);
    let s_val = (pwr * 10.0) as i32;
    let f_val = (spd * 10.0) as i32;
    let bed_size = 400.0;

    let center_parts: Vec<&str> = center_str.split(',').collect();
    let cx = if center_parts.len() == 2 { parse_dimension(center_parts[0])? } else { 200.0 };
    let cy = if center_parts.len() == 2 { parse_dimension(center_parts[1])? } else { 200.0 };

    let (intrinsic_w, intrinsic_h, intrinsic_min_x, intrinsic_min_y) = match shape.to_lowercase().as_str() {
        "square" => (50.0, 50.0, 0.0, 0.0),
        "heart" => (50.0, 37.5, 0.0, 0.0),
        other => {
            let path = if std::path::Path::new(other).exists() {
                other.to_string()
            } else {
                format!("assets/{}.svg", other)
            };
            
            if std::path::Path::new(&path).exists() {
                let (_, x1, y1, x2, y2) = svg_helper::load_svg_as_gcode(&path, 1.0, 0.0, 0.0, 0, 0)?;
                (x2 - x1, y2 - y1, x1, y1)
            } else {
                (1.0, 1.0, 0.0, 0.0)
            }
        }
    };

    if let Some(fit) = fit_str {
        let parts: Vec<&str> = fit.split('x').collect();
        if parts.len() == 2 {
            let target_w = parse_dimension(parts[0])?;
            let target_h = parse_dimension(parts[1])?;
            let scale_w = target_w / intrinsic_w;
            let scale_h = target_h / intrinsic_h;
            scale = scale_w.min(scale_h);
        }
    }

    let offset_x = cx - (intrinsic_min_x + intrinsic_w / 2.0) * scale;
    let offset_y = cy - (intrinsic_min_y + intrinsic_h / 2.0) * scale;

    let (path_gcode, min_x, min_y, max_x, max_y) = match shape.to_lowercase().as_str() {
        "square" => {
            let size = 50.0 * scale;
            let x2 = offset_x + size;
            let y2 = offset_y + size;
            (format!("M5\nG0 X{:.2} Y{:.2} F3000\nM4 S{} F{}\nG1 X{:.2}\nG1 Y{:.2}\nG1 X{:.2}\nG1 Y{:.2}\n", offset_x, offset_y, s_val, f_val, x2, y2, offset_x, offset_y), offset_x, offset_y, x2, y2)
        },
        "heart" => {
            let w = 50.0 * scale;
            let h = 37.5 * scale;
            let r = 12.5 * scale;
            let start_x = offset_x + (w/2.0);
            let x_right = offset_x + w;
            let y_mid = offset_y + (h * 0.66);
            (format!("M5\nG0 X{:.2} Y{:.2} F3000\nM4 S{} F{}\nG1 X{:.2} Y{:.2}\nG3 X{:.2} Y{:.2} R{:.2}\nG3 X{:.2} Y{:.2} R{:.2}\nG1 X{:.2} Y{:.2}\n", 
                start_x, offset_y, s_val, f_val, x_right, y_mid, start_x, y_mid, r, offset_x, y_mid, r, start_x, offset_y), offset_x, offset_y, x_right, y_mid + r)
        },
        other => {
            let path = if std::path::Path::new(other).exists() {
                other.to_string()
            } else {
                format!("assets/{}.svg", other)
            };
            
            if std::path::Path::new(&path).exists() {
                svg_helper::load_svg_as_gcode(&path, scale, offset_x, offset_y, s_val, f_val)?
            } else {
                return Err(format!("Unknown shape '{}'. Try Square, Heart, or a file in assets/.", shape).into());
            }
        }
    };

    if max_x > bed_size || max_y > bed_size || min_x < 0.0 || min_y < 0.0 {
        return Err(format!("Pattern out of bounds! Reaches ({:.1}, {:.1}) to ({:.1}, {:.1}) on {:.1}mm bed.", min_x, min_y, max_x, max_y, bed_size).into());
    }

    let mut final_gcode = String::new();
    final_gcode.push_str("G90\n$H\n"); 
    for _ in 0..passes {
        final_gcode.push_str(&path_gcode);
    }
    final_gcode.push_str("M5\n$H\n");

    Ok((final_gcode, format!("Dynamic {} (Scale: {:.2}x, Center: {:.1},{:.1}, Power: {}%, Speed: {}%)", shape, scale, cx, cy, pwr, spd)))
}

pub fn generate_image_gcode(path: &str, pwr_max: f32, speed: f32, scale: f32, passes: u32, fit: Option<(f32, f32)>, center: (f32, f32)) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let img = raylib::prelude::Image::load_image(path).map_err(|e| format!("Failed to load image: {}", e))?;
    
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

    let mut gcode = String::new();
    gcode.push_str("G90\n$H\n");
    
    let pixels = img.get_image_data();
    let f_val = (speed * 10.0) as i32;

    for _ in 0..passes {
        for y in 0..img.height() {
            let actual_y = offset_y + (img.height() - 1 - y) as f32 * final_scale;
            // Move to start of line
            gcode.push_str(&format!("M5\nG0 X{:.2} Y{:.2} F3000\n", offset_x, actual_y));
            gcode.push_str(&format!("M4 F{}\n", f_val));
            
            for x in 0..img.width() {
                let pixel_idx = (y * img.width() as i32 + x) as usize;
                let color = pixels[pixel_idx];
                // Luminance formula for more accurate grayscale conversion
                let luminance = 0.2126 * color.r as f32 + 0.7152 * color.g as f32 + 0.0722 * color.b as f32;
                // Invert: darker (lower luminance) -> higher power
                let intensity = 1.0 - (luminance / 255.0);
                let s_val = (intensity * pwr_max * 10.0) as i32;
                
                let actual_x = offset_x + x as f32 * final_scale;
                if s_val > 0 {
                    gcode.push_str(&format!("G1 X{:.2} S{}\n", actual_x, s_val));
                } else {
                    gcode.push_str(&format!("G0 X{:.2}\n", actual_x));
                }
            }
        }
    }
    gcode.push_str("M5\n$H\n");

    let filename = std::path::Path::new(path).file_name().and_then(|f| f.to_str()).unwrap_or("image");
    Ok((gcode, format!("Image {} (Scale: {:.2}x, Center: {:.1},{:.1}, Power: {}%, Speed: {}%)", filename, final_scale, center.0, center.1, pwr_max, speed)))
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

    println!("[{}] CLI: Mode = {} {}", get_ts(), label, if use_virtual { "(VIRTUAL)" } else { "" });

    let mut virtual_dev = if use_virtual { Some(VirtualDevice::new()) } else { None };
    let mut real_port = if !use_virtual {
        let port_name = "/dev/ttyUSB0";
        let baud_rate = 115200;
        let mut port = serialport::new(port_name, baud_rate)
            .timeout(std::time::Duration::from_millis(100))
            .open()?;
        println!("[{}] SERIAL: Connected to {}", get_ts(), port_name);
        // Clear any pending data
        let mut discard = vec![0u8; 1024];
        while let Ok(n) = port.read(discard.as_mut_slice()) {
            if n == 0 { break; }
        }
        Some(port)
    } else {
        None
    };

    for line in cmd_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        
        // Wait for Idle before Homing
        if trimmed.starts_with("$H") {
            let mut is_idle = false;
            let idle_start = std::time::Instant::now();
            println!("[{}] WAITING for Idle state before Homing...", get_ts());
            
            while idle_start.elapsed().as_secs() < 60 {
                if let Some(ref mut dev) = virtual_dev {
                    dev.update();
                    let res = dev.process_command("?");
                    if res.iter().any(|l| l.contains("Idle")) { is_idle = true; }
                } else if let Some(ref mut port) = real_port {
                    port.write_all(b"?")?;
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    let mut status_buf = vec![0u8; 1024];
                    if let Ok(n) = port.read(status_buf.as_mut_slice()) {
                        let status_line = String::from_utf8_lossy(&status_buf[..n]);
                        if status_line.contains("Idle") { is_idle = true; }
                    }
                }
                if is_idle { break; }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        let full_cmd = if trimmed == "0x18" { "\x18".to_string() } else { format!("{}\n", trimmed) };
        let explanation = decode_gcode(trimmed);
        
        if (trimmed.starts_with("M3") || trimmed.starts_with("M4")) && !trimmed.contains("S0") {
            println!("\x1b[1;31m########################################\x1b[0m");
            println!("\x1b[1;31m#            !!! LASER ON !!!          #\x1b[0m");
            println!("\x1b[1;31m########################################\x1b[0m");
        }

        println!("[{}] SEND: {:?} | Interpreter: {}", get_ts(), trimmed, explanation);
        
        let mut responses = Vec::new();
        if let Some(ref mut dev) = virtual_dev {
            responses = dev.process_command(trimmed);
            // If it started a move or homing, wait for it to finish to simulate realistic timing in CLI mode
            while dev.state == "Run" || dev.state == "Home" {
                std::thread::sleep(std::time::Duration::from_millis(20));
                dev.update();
            }
        } else if let Some(ref mut port) = real_port {
            port.write_all(full_cmd.as_bytes())?;
            let mut serial_buf: Vec<u8> = vec![0; 1024];
            let mut accumulator = String::new();
            let start_time = std::time::Instant::now();
            let timeout_secs = if trimmed.starts_with("$H") { 180 } else { 30 };
            let mut finished = false;

            while start_time.elapsed().as_secs() < timeout_secs {
                if let Ok(t) = port.read(serial_buf.as_mut_slice()) {
                    if t > 0 {
                        accumulator.push_str(&String::from_utf8_lossy(&serial_buf[..t]));
                        while let Some(pos) = accumulator.find('\n') {
                            let res_line = accumulator[..pos].trim().to_string();
                            accumulator.drain(..=pos);
                            if !res_line.is_empty() {
                                responses.push(res_line.clone());
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

        for res_line in responses {
            let explanation = decode_response(&res_line);
            println!("[{}] RECV: {:?} | Interpreter: {}", get_ts(), res_line, explanation);
        }
        
        if trimmed.starts_with("$H") {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    Ok(())
}
