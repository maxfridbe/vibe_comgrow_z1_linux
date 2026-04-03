pub fn decode_gcode(cmd: &str) -> String {
    let cmd = cmd.trim();
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
            "?" => "Status Report",
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
            "0x18" => "Soft Reset",
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
        l if l.starts_with("error:") => format!("Machine Error [{}]", &l[6..]),
        l if l.starts_with("ALARM:") => format!("Safety Alarm [{}]", &l[6..]),
        l if l.starts_with("Grbl") => "Firmware Greeting".to_string(),
        _ => trimmed.to_string(),
    }
}
