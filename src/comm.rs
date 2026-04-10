use crate::gcode::decode_response;
use crate::state::{AppState, LogEntry};
use crate::virtual_device::VirtualDevice;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::fs::{self, File};
use std::path::PathBuf;

fn get_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let ms = duration.subsec_millis();
    let hh = (secs / 3600) % 24;
    let mm = (secs / 60) % 60;
    let ss = secs % 60;
    format!("{:02}:{:02}:{:02}:{:03}", hh, mm, ss, ms)
}

fn get_full_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    duration.as_millis().to_string()
}

struct Logger {
    file: Option<File>,
}

impl Logger {
    fn new() -> Self {
        Self { file: None }
    }

    fn ensure_active(&mut self) {
        if self.file.is_some() { return; }
        
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let log_dir = PathBuf::from(home).join(".johnny5").join("burnlog");
        let _ = fs::create_dir_all(&log_dir);
        
        let timestamp = get_full_timestamp();
        let log_path = log_dir.join(format!("{}.log", timestamp));
        
        println!("[{}] LOGGING: Starting burn log at {:?}", get_timestamp(), log_path);
        if let Ok(f) = File::create(&log_path) {
            self.file = Some(f);
        }
    }

    fn log(&mut self, text: &str) {
        if let Some(ref mut f) = self.file {
            let _ = writeln!(f, "[{}] {}", get_timestamp(), text);
        }
    }

    fn close(&mut self) {
        self.file = None;
    }
}

pub fn start_serial_thread(state: Arc<Mutex<AppState>>, rx: Receiver<String>) {
    std::thread::spawn(move || {
        let baud_rate = 115200;
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut wait_for_ok = false;
        let mut virtual_machine = VirtualDevice::new();
        let mut logger = Logger::new();

        loop {
            let port_name = {
                let guard = state.lock().unwrap();
                guard.port.clone()
            };

            if port_name == "VIRTUAL" {
                let mut last_status_query = std::time::Instant::now();
                loop {
                    // Check if port changed back to real
                    let (port_changed, burn_log_active, is_idle) = {
                        let guard = state.lock().unwrap();
                        (guard.port != "VIRTUAL", guard.burn_log_active, guard.machine_state == "Idle")
                    };

                    if port_changed { break; }

                    if burn_log_active {
                        logger.ensure_active();
                    }

                    virtual_machine.update();

                    // Periodic Status Query (every 250ms for virtual to feel snappy)
                    if last_status_query.elapsed().as_millis() > 250 {
                        let responses = virtual_machine.process_command("?");
                        if burn_log_active { logger.log("TX: ?"); }
                        handle_responses(&state, responses, &mut wait_for_ok, false, &mut logger);
                        last_status_query = std::time::Instant::now();
                    }

                    // Receive from rx
                    while let Ok(cmd) = rx.try_recv() {
                        if cmd == "!" || cmd == "~" || cmd == "?" || cmd == "\x18" || cmd == "0x18" {
                            {
                                let mut guard = state.lock().unwrap();
                                guard.process_command_for_state(&cmd, true);
                            }
                            if burn_log_active { logger.log(&format!("TX: {}", cmd)); }
                            let responses = virtual_machine.process_command(&cmd);
                            handle_responses(&state, responses, &mut wait_for_ok, cmd == "?", &mut logger);
                            if cmd == "\x18" || cmd == "0x18" {
                                queue.clear();
                                wait_for_ok = false;
                            }
                        } else {
                            for line in cmd.lines() {
                                if !line.trim().is_empty() {
                                    queue.push_back(line.trim().to_string());
                                }
                            }
                        }
                    }

                    if !wait_for_ok {
                        if let Some(cmd) = queue.pop_front() {
                            {
                                let mut guard = state.lock().unwrap();
                                guard.process_command_for_state(&cmd, false);
                            }
                            if burn_log_active { logger.log(&format!("TX: {}", cmd)); }
                            wait_for_ok = true;
                            let responses = virtual_machine.process_command(&cmd);
                            handle_responses(&state, responses, &mut wait_for_ok, false, &mut logger);

                            // Simulate realistic timing by waiting for the virtual machine to finish its move/home
                            while virtual_machine.state == "Run" || virtual_machine.state == "Home" {
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                virtual_machine.update();
                                if last_status_query.elapsed().as_millis() > 250 {
                                    let responses = virtual_machine.process_command("?");
                                    if burn_log_active { logger.log("TX: ?"); }
                                    handle_responses(&state, responses, &mut wait_for_ok, false, &mut logger);
                                    last_status_query = std::time::Instant::now();
                                }
                            }
                        } else if burn_log_active && is_idle {
                            let mut guard = state.lock().unwrap();
                            guard.is_burning = false;
                            guard.burn_log_active = false;
                            logger.log("Session complete. Closing log.");
                            logger.close();
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            } else {
                if let Ok(mut port) =
                    serialport::new(&port_name, baud_rate).timeout(std::time::Duration::from_millis(10)).open()
                {
                    println!("[{}] SERIAL: Connected to {}", get_timestamp(), port_name);
                    {
                        let mut guard = state.lock().unwrap();
                        guard.serial_logs.push_back(LogEntry {
                            text: format!("Connected to {}", port_name),
                            explanation: format!("Baud rate: {}", baud_rate),
                            is_response: false,
                            timestamp: get_timestamp(),
                        });
                    }

                    let mut serial_buf: Vec<u8> = vec![0; 1024];
                    let mut line_accumulator = String::new();
                    let mut last_status_query = std::time::Instant::now();

                    loop {
                        // Check if port changed to virtual
                        let (port_changed, burn_log_active, is_idle) = {
                            let guard = state.lock().unwrap();
                            (guard.port != port_name, guard.burn_log_active, guard.machine_state == "Idle")
                        };

                        if port_changed { break; }

                        if burn_log_active {
                            logger.ensure_active();
                        }

                        // Periodic Status Query (every 500ms)
                        if last_status_query.elapsed().as_millis() > 500 {
                            let _ = port.write_all(b"?");
                            if burn_log_active { logger.log("TX: ?"); }
                            last_status_query = std::time::Instant::now();
                        }

                        // Receive from rx
                        while let Ok(cmd) = rx.try_recv() {
                            if cmd == "!" || cmd == "~" || cmd == "?" || cmd == "\x18" || cmd == "0x18" {
                                {
                                    let mut guard = state.lock().unwrap();
                                    guard.process_command_for_state(&cmd, true);
                                }
                                let actual_cmd = if cmd == "0x18" {
                                    "\x18"
                                } else {
                                    &cmd
                                };
                                let _ = port.write_all(actual_cmd.as_bytes());
                                if burn_log_active { logger.log(&format!("TX: {}", actual_cmd)); }
                                if actual_cmd == "\x18" {
                                    wait_for_ok = false;
                                    queue.clear();
                                }
                            } else {
                                queue.push_back(cmd);
                            }
                        }

                        if !wait_for_ok {
                            if let Some(cmd) = queue.pop_front() {
                                {
                                    let mut guard = state.lock().unwrap();
                                    guard.process_command_for_state(&cmd, false);
                                }
                                let full_cmd = format!("{}\n", cmd);
                                let _ = port.write_all(full_cmd.as_bytes());
                                if burn_log_active { logger.log(&format!("TX: {}", cmd)); }
                                wait_for_ok = true;
                            } else if burn_log_active && is_idle {
                                let mut guard = state.lock().unwrap();
                                guard.is_burning = false;
                                guard.burn_log_active = false;
                                logger.log("Session complete. Closing log.");
                                logger.close();
                            }
                        }

                        // Read responses
                        match port.read(serial_buf.as_mut_slice()) {
                            Ok(t) if t > 0 => {
                                line_accumulator.push_str(&String::from_utf8_lossy(&serial_buf[..t]));
                                while let Some(pos) = line_accumulator.find('\n') {
                                    let line = line_accumulator[..pos].trim().to_string();
                                    line_accumulator.drain(..=pos);
                                    if !line.is_empty() {
                                        let res_vec = vec![line];
                                        handle_responses(&state, res_vec, &mut wait_for_ok, false, &mut logger);
                                    }
                                }
                            }
                            Ok(_) => (),
                            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                            Err(_) => break,
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

fn handle_responses(state: &Arc<Mutex<AppState>>, responses: Vec<String>, wait_for_ok: &mut bool, force_log: bool, logger: &mut Logger) {
    let burn_log_active = { state.lock().unwrap().burn_log_active };
    for line in responses {
        if line.is_empty() {
            continue;
        }
        if burn_log_active {
            logger.log(&format!("RX: {}", line));
        }
        if line == "ok" || line.starts_with("error") || line.starts_with("Grbl") {
            *wait_for_ok = false;
        }

        let explanation = decode_response(&line);
        let mut guard = state.lock().unwrap();

        if line.starts_with('<') && line.contains('|') {
            let content = if line.ends_with('>') {
                &line[1..line.len() - 1]
            } else {
                &line[1..]
            };
            let parts: Vec<&str> = content.split('|').collect();
            if let Some(state_name) = parts.get(0) {
                guard.machine_state = state_name.to_string();
            }

            for part in &parts[1..] {
                if part.starts_with("MPos:") || part.starts_with("WPos:") {
                    let coords: Vec<&str> = part[5..].split(',').collect();
                    if coords.len() >= 2 {
                        let x = coords[0].parse::<f32>().unwrap_or(guard.machine_pos.x);
                        let y = coords[1].parse::<f32>().unwrap_or(guard.machine_pos.y);
                        guard.machine_pos = raylib::prelude::Vector2::new(x, y);
                    }
                }
            }
        }

        let is_periodic_status = line.starts_with('<') && line.contains('|');
        if !is_periodic_status || line.contains("Alarm") || line.contains("Hold") || force_log {
            guard.serial_logs.push_back(LogEntry {
                text: format!("RECV: {}", line),
                explanation,
                is_response: true,
                timestamp: get_timestamp(),
            });
            if guard.serial_logs.len() > 500 {
                guard.serial_logs.pop_front();
            }
        }
    }
}
