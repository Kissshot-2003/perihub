//! Un thread per dispositivo: tutte le richieste passano da qui, così l'animazione delle luci
//! e l'interfaccia non si accavallano mai sullo stesso collegamento HID.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use crate::error::{Error, Result};

pub trait DeviceTask: Send + 'static {
    /// Lavoro periodico (es. un fotogramma dell'effetto luci). Restituisce fra quanto richiamarlo.
    fn tick(&mut self) -> Result<Duration>;
}

type Job<T> = Box<dyn FnOnce(&mut T) + Send>;

pub struct Worker<T> {
    tx: Sender<Job<T>>,
    alive: Arc<AtomicBool>,
}

impl<T> Clone for Worker<T> {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone(), alive: self.alive.clone() }
    }
}

const CALL_TIMEOUT: Duration = Duration::from_secs(30);

impl<T: DeviceTask> Worker<T> {
    pub fn spawn(name: &str, mut task: T) -> Self {
        let (tx, rx) = mpsc::channel::<Job<T>>();
        let alive = Arc::new(AtomicBool::new(true));
        let flag = alive.clone();
        thread::Builder::new()
            .name(name.to_string())
            .spawn(move || {
                let mut wait = Duration::ZERO;
                loop {
                    match rx.recv_timeout(wait) {
                        Ok(job) => job(&mut task),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    match task.tick() {
                        Ok(next) => wait = next,
                        Err(error) => {
                            log::warn!("device unreachable: {error}");
                            break;
                        }
                    }
                }
                flag.store(false, Ordering::SeqCst);
            })
            .expect("failed to start the device thread");
        Self { tx, alive }
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// Esegue `f` nel thread del dispositivo e ne attende il risultato.
    pub fn call<R: Send + 'static>(&self, f: impl FnOnce(&mut T) -> Result<R> + Send + 'static) -> Result<R> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(Box::new(move |task| {
                let _ = reply_tx.send(f(task));
            }))
            .map_err(|_| Error::NotConnected)?;
        reply_rx.recv_timeout(CALL_TIMEOUT).map_err(|_| Error::Timeout)?
    }
}
