use std::time::{Instant, Duration};
use raylib::prelude::Vector2;

pub struct VirtualDevice {
    pub pos: Vector2,
    pub target_pos: Vector2,
    pub state: String,
    pub power: f32,
    pub feed_rate: f32,
    pub last_update: Instant,
    pub move_start: Option<Instant>,
    pub move_duration: Duration,
    pub arc_data: Option<ArcData>,
    pub homing_start: Option<Instant>,
    pub homing_start_pos: Vector2,
    pub is_absolute: bool,
    pub is_metric: bool,
    pub air_assist: bool,
}

pub struct ArcData {
    pub center: Vector2,
    pub radius: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub is_clockwise: bool,
}

impl VirtualDevice {
    pub fn new() -> Self {
        Self {
            pos: Vector2::new(0.0, 0.0),
            target_pos: Vector2::new(0.0, 0.0),
            state: "Idle".to_string(),
            power: 0.0,
            feed_rate: 1000.0,
            last_update: Instant::now(),
            move_start: None,
            move_duration: Duration::from_secs(0),
            arc_data: None,
            homing_start: None,
            homing_start_pos: Vector2::new(0.0, 0.0),
            is_absolute: true,
            is_metric: true,
            air_assist: false,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        
        if let Some(start) = self.homing_start {
            let elapsed = now.duration_since(start);
            let duration = Duration::from_secs(5);
            if elapsed >= duration {
                self.pos = Vector2::new(0.0, 0.0);
                self.target_pos = Vector2::new(0.0, 0.0);
                self.state = "Idle".to_string();
                self.homing_start = None;
            } else {
                self.state = "Home".to_string();
                let t = elapsed.as_secs_f32() / duration.as_secs_f32();
                self.pos = Vector2::new(
                    self.homing_start_pos.x * (1.0 - t),
                    self.homing_start_pos.y * (1.0 - t),
                );
            }
            return;
        }

        if let Some(start) = self.move_start {
            let elapsed = now.duration_since(start);
            if elapsed >= self.move_duration {
                self.pos = self.target_pos;
                self.state = "Idle".to_string();
                self.move_start = None;
                self.arc_data = None;
            } else {
                self.state = "Run".to_string();
                self.pos = self.current_interpolated_pos();
            }
        } else {
            if self.state != "Alarm" && self.state != "Hold" {
                self.state = "Idle".to_string();
            }
        }
    }

    fn current_interpolated_pos(&self) -> Vector2 {
        if let Some(start) = self.move_start {
            let elapsed = Instant::now().duration_since(start);
            let t = (elapsed.as_secs_f32() / self.move_duration.as_secs_f32()).min(1.0);
            
            if let Some(ref arc) = self.arc_data {
                let angle = arc.start_angle + t * (arc.end_angle - arc.start_angle);
                Vector2::new(
                    arc.center.x + arc.radius * angle.cos(),
                    arc.center.y + arc.radius * angle.sin(),
                )
            } else {
                Vector2::new(
                    self.pos.x + (self.target_pos.x - self.pos.x) * t,
                    self.pos.y + (self.target_pos.y - self.pos.y) * t,
                )
            }
        } else if let Some(start) = self.homing_start {
            let elapsed = Instant::now().duration_since(start);
            let t = (elapsed.as_secs_f32() / 5.0).min(1.0);
            Vector2::new(
                self.homing_start_pos.x * (1.0 - t),
                self.homing_start_pos.y * (1.0 - t),
            )
        } else {
            self.pos
        }
    }

    pub fn process_command(&mut self, cmd: &str) -> Vec<String> {
        let cmd = cmd.trim().to_uppercase();
        if cmd == "?" {
            let p = self.current_interpolated_pos();
            return vec![format!("<{}|MPos:{:.3},{:.3},0.000|FS:{:.0},{}|WCO:0.000,0.000,0.000>", 
                self.state, p.x, p.y, self.feed_rate, self.power as i32)];
        }
        
        if self.state == "Alarm" && cmd != "$X" && cmd != "0X18" && cmd != "\x18" {
            return vec!["error:9".to_string()]; // Alarm lock
        }

        if cmd == "$H" {
            self.homing_start = Some(Instant::now());
            self.homing_start_pos = self.pos;
            self.state = "Home".to_string();
            return vec!["ok".to_string()];
        }
        if cmd == "$X" {
            self.state = "Idle".to_string();
            return vec!["[MSG:Caution: Unlocked]".to_string(), "ok".to_string()];
        }
        if cmd == "!" {
            self.move_start = None;
            self.homing_start = None;
            self.state = "Hold".to_string();
            return vec!["ok".to_string()];
        }
        if cmd == "~" {
            if self.state == "Hold" {
                self.state = "Idle".to_string();
            }
            return vec!["ok".to_string()];
        }
        if cmd == "\x18" || cmd == "0x18" {
            self.pos = self.current_interpolated_pos();
            self.target_pos = self.pos;
            self.move_start = None;
            self.homing_start = None;
            self.arc_data = None;
            self.state = "Alarm".to_string();
            return vec!["Grbl 1.1h ['$' for help]".to_string(), "ok".to_string()];
        }

        // Parse G-code
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let mut new_x = None;
        let mut new_y = None;
        let mut new_r = None;
        let mut is_move = false;
        let mut is_arc = false;
        let mut is_clockwise = false;

        for p in parts {
            match p {
                "G0" | "G1" => is_move = true,
                "G2" => { is_move = true; is_arc = true; is_clockwise = true; },
                "G3" => { is_move = true; is_arc = true; is_clockwise = false; },
                "G90" => self.is_absolute = true,
                "G91" => self.is_absolute = false,
                "G21" => self.is_metric = true,
                "G20" => self.is_metric = false,
                "M3" | "M4" => { /* laser on handled by S */ },
                "M5" => self.power = 0.0,
                "M8" => self.air_assist = true,
                "M9" => self.air_assist = false,
                _ => {
                    if p.starts_with('X') { new_x = p[1..].parse::<f32>().ok(); }
                    else if p.starts_with('Y') { new_y = p[1..].parse::<f32>().ok(); }
                    else if p.starts_with('R') { new_r = p[1..].parse::<f32>().ok(); }
                    else if p.starts_with('F') { self.feed_rate = p[1..].parse::<f32>().unwrap_or(self.feed_rate); }
                    else if p.starts_with('S') { self.power = p[1..].parse::<f32>().unwrap_or(self.power); }
                }
            }
        }

        if is_move {
            let start_p = self.current_interpolated_pos();
            self.pos = start_p;
            let mut target = start_p;
            if self.is_absolute {
                if let Some(x) = new_x { target.x = x; }
                if let Some(y) = new_y { target.y = y; }
            } else {
                if let Some(x) = new_x { target.x += x; }
                if let Some(y) = new_y { target.y += y; }
            }
            self.target_pos = target;
            
            if is_arc && new_r.is_some() {
                let r = new_r.unwrap();
                let dx = target.x - start_p.x;
                let dy = target.y - start_p.y;
                let d2 = dx*dx + dy*dy;
                let d = d2.sqrt();
                
                if d > 0.0 && d <= 2.0 * r.abs() {
                    let h = ((r*r - d2/4.0).max(0.0)).sqrt();
                    let mut cx = (start_p.x + target.x) / 2.0;
                    let mut cy = (start_p.y + target.y) / 2.0;
                    
                    let multiplier = if (is_clockwise && r > 0.0) || (!is_clockwise && r < 0.0) { 1.0 } else { -1.0 };
                    cx += multiplier * h * dy / d;
                    cy -= multiplier * h * dx / d;
                    
                    let start_angle = (start_p.y - cy).atan2(start_p.x - cx);
                    let mut end_angle = (target.y - cy).atan2(target.x - cx);
                    
                    if is_clockwise {
                        if end_angle >= start_angle { end_angle -= 2.0 * std::f32::consts::PI; }
                    } else {
                        if end_angle <= start_angle { end_angle += 2.0 * std::f32::consts::PI; }
                    }
                    
                    self.arc_data = Some(ArcData {
                        center: Vector2::new(cx, cy),
                        radius: r.abs(),
                        start_angle,
                        end_angle,
                        is_clockwise,
                    });
                    
                    // Approximate arc length for timing
                    let arc_len = r.abs() * (end_angle - start_angle).abs();
                    let speed_mm_per_sec = self.feed_rate / 60.0;
                    let seconds = if speed_mm_per_sec > 0.0 { arc_len / speed_mm_per_sec } else { 0.0 };
                    self.move_duration = Duration::from_secs_f32(seconds.max(0.01));
                } else {
                    // Fallback to linear
                    let dist = d;
                    let speed_mm_per_sec = self.feed_rate / 60.0;
                    let seconds = if speed_mm_per_sec > 0.0 { dist / speed_mm_per_sec } else { 0.0 };
                    self.move_duration = Duration::from_secs_f32(seconds.max(0.01));
                }
            } else {
                let dist = ((self.target_pos.x - self.pos.x).powi(2) + (self.target_pos.y - self.pos.y).powi(2)).sqrt();
                let speed_mm_per_sec = self.feed_rate / 60.0;
                let seconds = if speed_mm_per_sec > 0.0 { dist / speed_mm_per_sec } else { 0.0 };
                self.move_duration = Duration::from_secs_f32(seconds.max(0.01));
            }

            self.move_start = Some(Instant::now());
            self.state = "Run".to_string();
        }

        vec!["ok".to_string()]
    }
}
