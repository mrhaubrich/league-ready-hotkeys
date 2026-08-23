#![cfg(windows)]

use super::{
    discover_lockfile, parse_lockfile, parse_ready_check, transport::LcuClient, ReadyCheck,
};
use crate::app::HotkeyAction;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerEvent {
    PollFinished { ready: Option<ReadyCheck> },
    ActionFinished { request_id: u64, succeeded: bool },
}

enum WorkerCommand {
    Poll,
    Action {
        request_id: u64,
        action: HotkeyAction,
    },
    Shutdown,
}

trait WorkerTransport: Send + 'static {
    fn poll(&mut self, runtime: &tokio::runtime::Runtime) -> Option<ReadyCheck>;
    fn action(&mut self, runtime: &tokio::runtime::Runtime, action: HotkeyAction) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LockfileSignature {
    length: u64,
    modified: SystemTime,
}

impl LockfileSignature {
    fn read(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        Some(Self {
            length: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

struct ClientCache {
    path: PathBuf,
    signature: LockfileSignature,
    client: LcuClient,
}

impl ClientCache {
    fn matches(&self, path: &Path, signature: LockfileSignature) -> bool {
        self.path == path && self.signature == signature
    }
}

#[derive(Default)]
struct LiveTransport {
    cache: Option<ClientCache>,
    cache_reuses: u64,
    client_builds: u64,
}

impl WorkerTransport for LiveTransport {
    fn poll(&mut self, runtime: &tokio::runtime::Runtime) -> Option<ReadyCheck> {
        let client = self.client(runtime)?;
        let result = runtime.block_on(client.ready_check());
        match result {
            Ok(payload) => payload.and_then(|payload| parse_ready_check(&payload).ok()),
            Err(_) => {
                self.cache = None;
                None
            }
        }
    }

    fn action(&mut self, runtime: &tokio::runtime::Runtime, action: HotkeyAction) -> bool {
        let result = match self.client(runtime) {
            Some(client) => runtime.block_on(async {
                match action {
                    HotkeyAction::Accept => client.accept().await,
                    HotkeyAction::Decline => client.decline().await,
                }
            }),
            None => return false,
        };
        if result.is_err() {
            self.cache = None;
            false
        } else {
            true
        }
    }
}

impl LiveTransport {
    fn client(&mut self, _runtime: &tokio::runtime::Runtime) -> Option<&LcuClient> {
        if let Some(cache) = self.cache.as_ref() {
            if LockfileSignature::read(&cache.path)
                .is_some_and(|signature| cache.matches(&cache.path, signature))
            {
                self.cache_reuses += 1;
                return self.cache.as_ref().map(|cache| &cache.client);
            }
        }

        let path = discover_lockfile().ok()?;
        let signature = LockfileSignature::read(&path)?;
        let contents = fs::read_to_string(&path).ok()?;
        let credentials = parse_lockfile(&contents).ok()?;
        let client = LcuClient::new(&credentials).ok()?;
        self.client_builds += 1;
        self.cache = Some(ClientCache {
            path,
            signature,
            client,
        });
        self.cache.as_ref().map(|cache| &cache.client)
    }
}

impl LiveTransport {
    fn cache_diagnostic(&self) -> (u64, u64) {
        (self.cache_reuses, self.client_builds)
    }
}

pub struct LcuWorker {
    commands: mpsc::Sender<WorkerCommand>,
    events: mpsc::Receiver<WorkerEvent>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LcuWorker {
    pub fn spawn(notify: impl Fn() + Send + 'static) -> Self {
        Self::spawn_with_transport(LiveTransport::default(), notify)
    }

    fn spawn_with_transport(
        transport: impl WorkerTransport,
        notify: impl Fn() + Send + 'static,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let stopping = Arc::new(AtomicBool::new(false));
        let thread_stopping = Arc::clone(&stopping);
        let thread = thread::Builder::new()
            .name("lcu-worker".to_owned())
            .spawn(move || {
                run_worker(transport, command_rx, event_tx, thread_stopping, notify);
            })
            .expect("spawn LCU worker");
        Self {
            commands: command_tx,
            events: event_rx,
            stopping,
            thread: Some(thread),
        }
    }

    pub fn request_poll(&self) -> bool {
        !self.stopping.load(Ordering::Acquire) && self.commands.send(WorkerCommand::Poll).is_ok()
    }

    pub fn request_action(&self, request_id: u64, action: HotkeyAction) -> bool {
        !self.stopping.load(Ordering::Acquire)
            && self
                .commands
                .send(WorkerCommand::Action { request_id, action })
                .is_ok()
    }

    pub fn try_recv(&self) -> Option<WorkerEvent> {
        self.events.try_recv().ok()
    }

    pub fn shutdown(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.stopping.store(true, Ordering::Release);
        let _ = self.commands.send(WorkerCommand::Shutdown);
        let _ = thread.join();
    }
}

impl Drop for LcuWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_worker(
    mut transport: impl WorkerTransport,
    commands: mpsc::Receiver<WorkerCommand>,
    events: mpsc::Sender<WorkerEvent>,
    stopping: Arc<AtomicBool>,
    notify: impl Fn(),
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("create LCU worker runtime");
    while let Ok(command) = commands.recv() {
        if stopping.load(Ordering::Acquire) || matches!(command, WorkerCommand::Shutdown) {
            break;
        }
        let event = match command {
            WorkerCommand::Poll => WorkerEvent::PollFinished {
                ready: transport.poll(&runtime),
            },
            WorkerCommand::Action { request_id, action } => WorkerEvent::ActionFinished {
                request_id,
                succeeded: transport.action(&runtime, action),
            },
            WorkerCommand::Shutdown => break,
        };
        if stopping.load(Ordering::Acquire) {
            break;
        }
        if events.send(event).is_err() {
            break;
        }
        notify();
    }
}

struct FixedClientTransport {
    client: LcuClient,
}

impl WorkerTransport for FixedClientTransport {
    fn poll(&mut self, runtime: &tokio::runtime::Runtime) -> Option<ReadyCheck> {
        let _ = runtime.block_on(self.client.ready_check());
        None
    }

    fn action(&mut self, runtime: &tokio::runtime::Runtime, action: HotkeyAction) -> bool {
        runtime
            .block_on(async {
                match action {
                    HotkeyAction::Accept => self.client.accept().await,
                    HotkeyAction::Decline => self.client.decline().await,
                }
            })
            .is_ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StalledTransportDiagnostic {
    pub submission_latency: Duration,
    pub response_latency: Duration,
    pub ui_iterations: u64,
}

pub fn run_stalled_transport_diagnostic() -> Result<StalledTransportDiagnostic, String> {
    use super::LcuCredentials;
    use std::net::TcpListener;

    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|error| error.to_string())?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let (server_stop_tx, server_stop_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            let _stream = stream;
            let _ = server_stop_rx.recv_timeout(Duration::from_secs(5));
        }
    });
    let credentials = LcuCredentials {
        port,
        password: "diagnostic-only".to_owned(),
        protocol: "https".to_owned(),
    };
    let client = LcuClient::new(&credentials).map_err(|error| error.to_string())?;
    let (wake_tx, wake_rx) = mpsc::channel();
    let mut worker = LcuWorker::spawn_with_transport(FixedClientTransport { client }, move || {
        let _ = wake_tx.send(());
    });

    let submitted = Instant::now();
    if !worker.request_poll() {
        return Err("worker rejected diagnostic poll".to_owned());
    }
    let submission_latency = submitted.elapsed();
    let mut ui_iterations = 0;
    let response_started = Instant::now();
    while wake_rx.try_recv().is_err() && response_started.elapsed() < Duration::from_secs(3) {
        ui_iterations += 1;
        thread::yield_now();
    }
    let response_latency = response_started.elapsed();
    let event = worker.try_recv();
    worker.shutdown();
    let _ = server_stop_tx.send(());
    let _ = server.join();

    if submission_latency > Duration::from_millis(50) {
        return Err("poll submission blocked the caller".to_owned());
    }
    if !matches!(event, Some(WorkerEvent::PollFinished { ready: None })) {
        return Err("stalled poll did not finish within its timeout".to_owned());
    }
    if ui_iterations == 0 {
        return Err("caller made no progress while transport was stalled".to_owned());
    }
    Ok(StalledTransportDiagnostic {
        submission_latency,
        response_latency,
        ui_iterations,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheDiagnostic {
    pub client_builds: u64,
    pub cache_reuses: u64,
}

pub fn run_cache_diagnostic() -> Result<CacheDiagnostic, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let mut transport = LiveTransport::default();
    let _ = transport.poll(&runtime);
    let _ = transport.poll(&runtime);
    let (cache_reuses, client_builds) = transport.cache_diagnostic();
    if client_builds == 0 {
        return Err(
            "League client was not reachable; run this diagnostic while League is running"
                .to_owned(),
        );
    }
    if cache_reuses == 0 {
        return Err("the second poll did not reuse the cached LCU client".to_owned());
    }
    Ok(CacheDiagnostic {
        client_builds,
        cache_reuses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcu::LcuCredentials;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct StallingTransport {
        started: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }

    impl WorkerTransport for StallingTransport {
        fn poll(&mut self, _runtime: &tokio::runtime::Runtime) -> Option<ReadyCheck> {
            let _ = self.started.send(());
            let _ = self.release.recv();
            None
        }

        fn action(&mut self, _runtime: &tokio::runtime::Runtime, _action: HotkeyAction) -> bool {
            false
        }
    }

    struct CountingTransport {
        actions: Arc<AtomicUsize>,
    }

    impl WorkerTransport for CountingTransport {
        fn poll(&mut self, _runtime: &tokio::runtime::Runtime) -> Option<ReadyCheck> {
            None
        }

        fn action(&mut self, _runtime: &tokio::runtime::Runtime, _action: HotkeyAction) -> bool {
            self.actions.fetch_add(1, Ordering::AcqRel);
            false
        }
    }

    #[test]
    fn stalled_transport_does_not_block_command_caller() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (wake_tx, wake_rx) = mpsc::channel();
        let mut worker = LcuWorker::spawn_with_transport(
            StallingTransport {
                started: started_tx,
                release: release_rx,
            },
            move || {
                let _ = wake_tx.send(());
            },
        );
        let submitted = Instant::now();
        assert!(worker.request_poll());
        assert!(submitted.elapsed() < Duration::from_millis(50));
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(worker.try_recv().is_none());
        release_tx.send(()).unwrap();
        wake_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(matches!(
            worker.try_recv(),
            Some(WorkerEvent::PollFinished { ready: None })
        ));
        worker.shutdown();
    }

    #[test]
    fn failed_explicit_action_is_attempted_once_without_retry() {
        let actions = Arc::new(AtomicUsize::new(0));
        let (wake_tx, wake_rx) = mpsc::channel();
        let mut worker = LcuWorker::spawn_with_transport(
            CountingTransport {
                actions: Arc::clone(&actions),
            },
            move || {
                let _ = wake_tx.send(());
            },
        );
        assert!(worker.request_action(7, HotkeyAction::Decline));
        wake_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(
            worker.try_recv(),
            Some(WorkerEvent::ActionFinished {
                request_id: 7,
                succeeded: false
            })
        );
        thread::sleep(Duration::from_millis(25));
        assert_eq!(actions.load(Ordering::Acquire), 1);
        worker.shutdown();
    }

    #[test]
    fn shutdown_skips_an_action_queued_behind_stalled_work() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let actions = Arc::new(AtomicUsize::new(0));
        struct ShutdownTransport {
            stall: StallingTransport,
            actions: Arc<AtomicUsize>,
        }
        impl WorkerTransport for ShutdownTransport {
            fn poll(&mut self, runtime: &tokio::runtime::Runtime) -> Option<ReadyCheck> {
                self.stall.poll(runtime)
            }
            fn action(
                &mut self,
                _runtime: &tokio::runtime::Runtime,
                _action: HotkeyAction,
            ) -> bool {
                self.actions.fetch_add(1, Ordering::AcqRel);
                true
            }
        }
        let mut worker = LcuWorker::spawn_with_transport(
            ShutdownTransport {
                stall: StallingTransport {
                    started: started_tx,
                    release: release_rx,
                },
                actions: Arc::clone(&actions),
            },
            || {},
        );
        assert!(worker.request_poll());
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(worker.request_action(9, HotkeyAction::Accept));
        worker.stopping.store(true, Ordering::Release);
        release_tx.send(()).unwrap();
        worker.shutdown();
        assert_eq!(actions.load(Ordering::Acquire), 0);
    }

    #[test]
    fn lockfile_identity_requires_same_path_and_metadata() {
        let signature = LockfileSignature {
            length: 42,
            modified: SystemTime::UNIX_EPOCH,
        };
        let cache = ClientCache {
            path: PathBuf::from("C:\\League\\lockfile"),
            signature,
            client: LcuClient::new(&LcuCredentials {
                port: 1234,
                password: "test-only".to_owned(),
                protocol: "https".to_owned(),
            })
            .unwrap(),
        };
        assert!(cache.matches(Path::new("C:\\League\\lockfile"), signature));
        assert!(!cache.matches(Path::new("C:\\Other\\lockfile"), signature));
        assert!(!cache.matches(
            Path::new("C:\\League\\lockfile"),
            LockfileSignature {
                length: 43,
                ..signature
            }
        ));
    }
}
