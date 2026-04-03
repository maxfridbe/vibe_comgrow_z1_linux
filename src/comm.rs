use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::collections::VecDeque;
use std::io::{Write, Read};
use crate::state::{AppState, LogEntry};
use crate::gcode::decode_response;

pub fn start_serial_thread(state: Arc<Mutex<AppState>>, rx: Receiver<String>) {
    std::thread::spawn(move || {
        let port_name = "/dev/ttyUSB0";
        let baud_rate = 115200;
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut wait_for_ok = false;

        loop {
            if let Ok(mut port) = serialport::new(port_name, baud_rate)
                .timeout(std::time::Duration::from_millis(10))
                .open()
            {
                let mut serial_buf: Vec<u8> = vec![0; 1024];
                loop {
                    // Receive from rx and push to queue
                    while let Ok(cmd) = rx.try_recv() {
                        queue.push_back(cmd);
                    }

                    // Send if not waiting
                    if !wait_for_ok {
                        if let Some(cmd) = queue.pop_front() {
                            let full_cmd = format!("{}\n", cmd);
                            let _ = port.write_all(full_cmd.as_bytes());
                            wait_for_ok = true;
                        }
                    }

                    // Read responses
                    match port.read(serial_buf.as_mut_slice()) {
                        Ok(t) => {
                            let response = String::from_utf8_lossy(&serial_buf[..t]).to_string();
                            for line in response.lines() {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    if trimmed == "ok" || trimmed.starts_with("error") {
                                        wait_for_ok = false;
                                    }

                                    let explanation = decode_response(trimmed);

                                    let mut guard = state.lock().unwrap();
                                    guard.serial_logs.push(LogEntry {
                                        text: format!("Response: {}", trimmed),
                                        explanation,
                                    });
                                    if guard.serial_logs.len() > 100 {
                                        guard.serial_logs.remove(0);
                                    }
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                        Err(_) => break, // Reconnect on other errors
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1)); // Wait before retry
        }
    });
}
