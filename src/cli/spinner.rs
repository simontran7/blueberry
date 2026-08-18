use std::io::{self, IsTerminal, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const TICK: Duration = Duration::from_millis(80);

const BOLD_BLUE: &str = "\x1b[1;34m";
const RESET: &str = "\x1b[0m";
const ERASE_LINE: &str = "\x1b[2K\r";

pub(crate) struct Spinner {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    verb: &'static str,
    message: String,
}

impl Spinner {
    pub(crate) fn start(verb: &'static str, message: impl Into<String>) -> Self {
        let message = message.into();
        let stop = Arc::new(AtomicBool::new(false));
        let handle = if io::stdout().is_terminal() {
            Some(thread::spawn({
                let stop = Arc::clone(&stop);
                let message = message.clone();
                move || {
                    let mut stdout = io::stdout();
                    let mut frame_idx = 0;
                    while !stop.load(Ordering::Relaxed) {
                        let _ = write!(
                            stdout,
                            "{ERASE_LINE}{BOLD_BLUE}{} {verb}{RESET} {message}",
                            FRAMES[frame_idx],
                        );
                        let _ = stdout.flush();
                        frame_idx = (frame_idx + 1) % FRAMES.len();
                        thread::sleep(TICK);
                    }
                    let _ = stdout.write_all(ERASE_LINE.as_bytes());
                    let _ = stdout.flush();
                }
            }))
        } else {
            None
        };

        Self {
            stop,
            handle,
            verb,
            message,
        }
    }

    pub(crate) fn stop(&mut self) {
        if self.stop.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        Self::print_status(self.verb, &self.message);
    }

    pub(crate) fn print_status(verb: &str, message: &str) {
        if io::stdout().is_terminal() {
            println!("{BOLD_BLUE}{verb}{RESET} {message}");
        } else {
            println!("{verb} {message}");
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}
