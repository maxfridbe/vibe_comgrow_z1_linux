pub fn decode_gcode(cmd: &str) -> String {
    let cmd = cmd.trim();

    if cmd == "\x18" || cmd == "0x18" {
        return "Emergency Soft Reset (Ctrl-X)".to_string();
    }
    if cmd == "?" {
        return "Request Real-Time Status Report".to_string();
    }

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    
    let decoded = if cmd.starts_with("$J=") {
        let mut x = None;
        let mut y = None;
        for part in &parts {
            let p = if part.starts_with("$J=") { &part[3..] } else { *part };
            if p.starts_with('X') { x = Some(&p[1..]); }
            else if p.starts_with('Y') { y = Some(&p[1..]); }
        }
        if let Some(val) = x {
            let sign = if val.starts_with('-') { "" } else { "+" };
            format!("Jog X {}{}mm", sign, val)
        } else if let Some(val) = y {
            let sign = if val.starts_with('-') { "" } else { "+" };
            format!("Jog Y {}{}mm", sign, val)
        } else {
            "Jog Move".to_string()
        }
    } else if cmd.starts_with('$') && cmd.contains('=') {
        let clean_cmd = cmd.strip_prefix('$').unwrap_or(cmd);
        let mut parts = clean_cmd.split('=');
        if let (Some(setting), Some(value)) = (parts.next(), parts.next()) {
            match setting {
                "33" => format!("Set Dynamic Power Mode to {}", value),
                "100" => format!("Update X Steps/mm to {}", value),
                "101" => format!("Update Y Steps/mm to {}", value),
                "102" => format!("Update Z Steps/mm to {}", value),
                "110" => format!("Set X Max Rate to {} mm/min", value),
                "111" => format!("Set Y Max Rate to {} mm/min", value),
                "120" => format!("Set X Acceleration to {} mm/sec^2", value),
                "121" => format!("Set Y Acceleration to {} mm/sec^2", value),
                "130" => format!("Set X Max Travel to {}mm", value),
                "131" => format!("Set Y Max Travel to {}mm", value),
                "30" => format!("Set Max Spindle/Laser Speed (S) to {}", value),
                "32" => format!("Set Laser Mode to {} (0=Off, 1=On)", value),
                "20" => format!("Set Soft Limits to {} (0=Off, 1=On)", value),
                "21" => format!("Set Hard Limits to {} (0=Off, 1=On)", value),
                "22" => format!("Set Homing Cycle to {} (0=Off, 1=On)", value),
                _ => format!("Update GRBL Setting ${}={}", setting, value),
            }
        } else {
            "Update GRBL Setting".to_string()
        }
    } else if parts.iter().any(|p| p.starts_with("G1")) {
        let mut x = None;
        let mut y = None;
        let mut f = None;
        let mut s = None;
        for part in &parts {
            if part.starts_with('X') { x = Some(&part[1..]); }
            else if part.starts_with('Y') { y = Some(&part[1..]); }
            else if part.starts_with('F') { f = Some(&part[1..]); }
            else if part.starts_with('S') { s = Some(&part[1..]); }
        }
        let mut pos = Vec::new();
        if let Some(xv) = x { pos.push(format!("X{}", xv)); }
        if let Some(yv) = y { pos.push(format!("Y{}", yv)); }
        let pos_str = if pos.is_empty() { "".to_string() } else { format!("to {}", pos.join(" ")) };
        
        let mut params = Vec::new();
        if let Some(fv) = f { params.push(format!("F{}", fv)); }
        if let Some(sv) = s { params.push(format!("S{}", sv)); }
        let params_str = if params.is_empty() { "".to_string() } else { format!("({})", params.join(", ")) };
        
        format!("Burn Linear {} {}", pos_str, params_str)
    } else if parts.iter().any(|p| p.starts_with("G0")) {
        let mut x = None;
        let mut y = None;
        for part in &parts {
            if part.starts_with('X') { x = Some(&part[1..]); }
            else if part.starts_with('Y') { y = Some(&part[1..]); }
        }
        let mut pos = Vec::new();
        if let Some(xv) = x { pos.push(format!("X{}", xv)); }
        if let Some(yv) = y { pos.push(format!("Y{}", yv)); }
        let pos_str = if pos.is_empty() { "".to_string() } else { format!("to {}", pos.join(" ")) };
        
        format!("Jump {}", pos_str)
    } else if parts.iter().any(|p| p.starts_with("M3")) || parts.iter().any(|p| p.starts_with("M4")) {
        let is_m3 = parts.iter().any(|p| p.starts_with("M3"));
        let label = if is_m3 { "Laser Constant On" } else { "Laser Dynamic On" };
        let mut s = None;
        for part in &parts {
            if part.starts_with('S') { s = Some(&part[1..]); }
        }
        if let Some(sv) = s {
            format!("{} (Power: {})", label, sv)
        } else {
            label.to_string()
        }
    } else {
        match cmd {
            "$H" => "Home Machine",
            "M5" => "Laser Off",
            "!" => "Feed Hold",
            "~" => "Cycle Start",
            "$X" => "Kill Alarm",
            "G90" => "Absolute Distance",
            "G91" => "Incremental Distance",
            "G21" => "Millimeter Units",
            "G20" => "Inch Units",
            "G92 X0 Y0" => "Set Origin",
            "M8" => "Air Assist On",
            "M9" => "Air Assist Off",
            c if c.starts_with("$") => "Settings Change",
            _ => "G-Code Command",
        }.to_string()
    };

    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn decode_response(resp: &str) -> String {
    let trimmed = resp.trim();
    match trimmed {
        "ok" => "Success / OK".to_string(),
        l if l.starts_with("error:15") => "Jog target exceeds machine travel. (Check limits or Home machine)".to_string(),
        l if l.starts_with("error:") => format!("Machine Error [{}]", &l[6..]),
        l if l.starts_with("ALARM:") => format!("Safety Alarm [{}]. You must Reset or Unlock.", &l[6..]),
        l if l.starts_with('<') && l.ends_with('>') => {
            let content = &l[1..l.len() - 1];
            let parts: Vec<&str> = content.split('|').collect();
            let state = parts.get(0).cloned().unwrap_or("Unknown");
            
            let mut info = match state {
                "Idle" => "Machine is IDLE and ready for commands.".to_string(),
                "Run" => "Machine is MOVING / EXECUTING commands.".to_string(),
                "Alarm" => "Machine in ALARM state (Unlock Required).".to_string(),
                "Hold" => "Machine is in HOLD (Resume Required).".to_string(),
                _ => format!("Machine State: {}", state),
            };
            
            for part in &parts[1..] {
                if part.starts_with("MPos:") {
                    let coords: Vec<&str> = part[5..].split(',').collect();
                    if coords.len() >= 2 {
                        info.push_str(&format!(" | Pos: X{} Y{}", coords[0], coords[1]));
                    }
                } else if part.starts_with("FS:") {
                    let speeds: Vec<&str> = part[3..].split(',').collect();
                    if speeds.len() >= 2 {
                        info.push_str(&format!(" | Feed: {} Spindle: {}", speeds[0], speeds[1]));
                    }
                }
            }
            info
        },
        l if l.contains("Caution: Unlocked") => "Machine has been safely UNLOCKED.".to_string(),
        l if l.contains("Homing cycle") => "Machine is performing a HOMING cycle...".to_string(),
        l if l.starts_with("Grbl") => "Firmware Greeting".to_string(),
        _ => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_gcode_settings() {
        assert_eq!(decode_gcode("$100=80.0"), "Update X Steps/mm to 80.0");
        assert_eq!(decode_gcode("$101=80.0"), "Update Y Steps/mm to 80.0");
        assert_eq!(decode_gcode("$102=250.0"), "Update Z Steps/mm to 250.0");
        assert_eq!(decode_gcode("$130=400"), "Set X Max Travel to 400mm");
        assert_eq!(decode_gcode("$131=400"), "Set Y Max Travel to 400mm");
        assert_eq!(decode_gcode("$30=1000"), "Set Max Spindle/Laser Speed (S) to 1000");
        assert_eq!(decode_gcode("$32=1"), "Set Laser Mode to 1 (0=Off, 1=On)");
        assert_eq!(decode_gcode("$20=0"), "Set Soft Limits to 0 (0=Off, 1=On)");
        assert_eq!(decode_gcode("$21=0"), "Set Hard Limits to 0 (0=Off, 1=On)");
        assert_eq!(decode_gcode("$1=25"), "Update GRBL Setting");
    }

    #[test]
    fn test_decode_gcode_status() {
        assert_eq!(decode_gcode("?"), "Request Real-Time Status Report");
    }

    #[test]
    fn test_decode_gcode_reset() {
        assert_eq!(decode_gcode("\x18"), "Emergency Soft Reset (Ctrl-X)");
        assert_eq!(decode_gcode("0x18"), "Emergency Soft Reset (Ctrl-X)");
    }

    #[test]
    fn test_decode_gcode_linear() {
        assert_eq!(decode_gcode("G1 X10 Y20 F1000 S500"), "Burn Linear to X10 Y20 (F1000, S500)");
    }
}
