//! PTY handler for spawning and communicating with a shell

use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;

/// Maximum number of pending PTY output chunks
const PTY_CHANNEL_CAPACITY: usize = 256;

/// PTY handler for spawning and communicating with a shell
pub struct PtyHandler {
    #[allow(dead_code)]
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output_rx: Receiver<Vec<u8>>,
    _reader_thread: thread::JoinHandle<()>,
    is_dead: bool,
}

impl PtyHandler {
    /// Create a new PTY with default shell
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        Self::with_command(cols, rows, None)
    }

    /// Create a new PTY with custom command
    pub fn with_command(cols: u16, rows: u16, command: Option<&str>) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        let mut cmd = if let Some(cmd) = command {
            CommandBuilder::new(cmd)
        } else {
            CommandBuilder::new_default_prog()
        };
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let _child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell")?;

        drop(pair.slave);

        let master = pair.master;
        let mut reader = master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;
        let writer = master.take_writer().context("Failed to take PTY writer")?;

        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(PTY_CHANNEL_CAPACITY);

        let reader_thread = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("PTY read error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            master,
            writer,
            output_rx: rx,
            _reader_thread: reader_thread,
            is_dead: false,
        })
    }

    /// Resize the PTY
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to resize PTY")?;
        Ok(())
    }

    /// Write data to the PTY
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer
            .write_all(data)
            .context("Failed to write to PTY")?;
        self.writer.flush().context("Failed to flush PTY writer")?;
        Ok(())
    }

    /// Try to read available output (non-blocking)
    pub fn try_read(&mut self) -> Option<Vec<u8>> {
        match self.output_rx.try_recv() {
            Ok(data) => Some(data),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.is_dead = true;
                None
            }
        }
    }

    /// Check if the PTY is still alive
    pub fn is_alive(&self) -> bool {
        !self.is_dead
    }
}

/// Shared PTY handler
pub type SharedPty = Arc<Mutex<PtyHandler>>;

/// Create a shared PTY
pub fn create_shared_pty(cols: u16, rows: u16) -> Result<SharedPty> {
    Ok(Arc::new(Mutex::new(PtyHandler::new(cols, rows)?)))
}
