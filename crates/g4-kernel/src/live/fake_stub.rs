//! A fake of the melonDS 1.1 ARM9 GDB stub on `127.0.0.1:0`, for tests.
//!
//! Uses `std` only (no `crate::` imports): `tests/g4k_live.rs` includes this file with
//! `#[path = "../src/live/fake_stub.rs"]`, and the studio crate uses it through the
//! `test-support` feature.
//!
//! Faithful to the quirks measured in `docs/research/2026-10-30-phase-3d/probe-gdb-stub.md`:
//! - a client that is served halts the emulation until `c`/`vCont;c`; a disconnect lets it run;
//! - the client must send `+` within [`Quirks::ack_deadline`] after being served, else the stub
//!   closes the connection;
//! - `m` up to [`Quirks::max_read`] bytes; one byte more gets no reply at all, more than that `E02`;
//! - a packet longer than [`Quirks::wedge_above`] characters wedges the stub: it never answers
//!   any client again;
//! - with [`Quirks::one_client`] a second client is accepted but stays unanswered until the
//!   first one disconnects;
//! - `QStartNoAckMode` is not reset by a disconnect;
//! - stop replies are `S02` (`?`) and `S05` (`\x03`).
//!
//! RAM is the 4 MiB main RAM at `0x02000000`; `m`/`M` outside `[0x02000000, 0x02400000)` get `E01`.
//!
//! The log records the handshake `+`, every received packet payload verbatim and the interrupt
//! byte as `"\x03"`; acks after the handshake are not logged.
//!
//! All threads end when the [`FakeStub`] is dropped (the drop joins them) or, per connection,
//! when the client disconnects.

#![allow(dead_code)]

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Start of the emulated main RAM.
pub const RAM_BASE: u32 = 0x0200_0000;
/// Size of the emulated main RAM (4 MiB).
pub const RAM_SIZE: usize = 0x0040_0000;
/// The `qSupported` reply of melonDS 1.1.
pub const FEATURES: &str = "PacketSize=47F;qXfer:features:read+;swbreak-;hwbreak+;QStartNoAckMode+";

/// How often the stub's threads look at the stop flag.
const POLL: Duration = Duration::from_millis(10);

/// Behaviour switches of the fake. `Default` is [`Quirks::melonds_1_1`].
#[derive(Debug, Clone)]
pub struct Quirks {
    /// Time a served client has to send its first `+` before the stub disconnects.
    pub ack_deadline: Duration,
    /// Largest `m` length that is answered; `max_read + 1` gets no reply, more gets `E02`.
    pub max_read: usize,
    /// A packet (`$…#xx`) longer than this many characters wedges the stub for good.
    pub wedge_above: usize,
    /// `false`: accepts connections and reads, but never sends a single byte.
    pub answer: bool,
    /// Serve one client at a time; further clients are accepted but stay unanswered.
    pub one_client: bool,
}

impl Quirks {
    /// The behaviour measured with melonDS 1.1.
    pub fn melonds_1_1() -> Quirks {
        Quirks {
            ack_deadline: Duration::from_millis(1000),
            max_read: 575,
            wedge_above: 1150,
            answer: true,
            one_client: true,
        }
    }
}

impl Default for Quirks {
    fn default() -> Quirks {
        Quirks::melonds_1_1()
    }
}

/// A running fake stub; dropping it closes every connection and ends its threads.
pub struct FakeStub {
    pub port: u16,
    /// 4 MiB main RAM at [`RAM_BASE`]; tests write here to play "the game changes a value".
    pub ram: Arc<Mutex<Vec<u8>>>,
    /// Whether the emulation is halted (a client is served and has not sent `c`).
    pub halted: Arc<AtomicBool>,
    pub log: Arc<Mutex<Vec<String>>>,
    /// Set once a too long packet arrived: the stub never answers again.
    pub wedged: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
}

/// State shared by the accept thread and all connection threads.
#[derive(Clone)]
struct Shared {
    quirks: Quirks,
    ram: Arc<Mutex<Vec<u8>>>,
    halted: Arc<AtomicBool>,
    log: Arc<Mutex<Vec<String>>>,
    wedged: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    /// A client is being served (only used with `one_client`).
    serving: Arc<AtomicBool>,
    /// `QStartNoAckMode` was accepted; like 1.1 it survives disconnects.
    no_ack: Arc<AtomicBool>,
}

impl FakeStub {
    /// Binds `127.0.0.1:0` and starts accepting. Panics if the socket cannot be bound (tests only).
    pub fn start(quirks: Quirks) -> FakeStub {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the fake stub");
        listener
            .set_nonblocking(true)
            .expect("non-blocking listener");
        let port = listener.local_addr().expect("local address").port();
        let shared = Shared {
            quirks,
            ram: Arc::new(Mutex::new(vec![0; RAM_SIZE])),
            halted: Arc::new(AtomicBool::new(false)),
            log: Arc::new(Mutex::new(Vec::new())),
            wedged: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            serving: Arc::new(AtomicBool::new(false)),
            no_ack: Arc::new(AtomicBool::new(false)),
        };
        let mut stub = FakeStub {
            port,
            ram: shared.ram.clone(),
            halted: shared.halted.clone(),
            log: shared.log.clone(),
            wedged: shared.wedged.clone(),
            stop: shared.stop.clone(),
            accept_thread: None,
        };
        stub.accept_thread = Some(std::thread::spawn(move || accept_loop(listener, shared)));
        stub
    }

    /// `127.0.0.1:<port>`.
    pub fn addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Writes `bytes` into RAM at the absolute address `addr` (the game changes a value).
    pub fn poke(&self, addr: u32, bytes: &[u8]) {
        let start = (addr - RAM_BASE) as usize;
        self.ram.lock().unwrap()[start..start + bytes.len()].copy_from_slice(bytes);
    }

    /// Reads `len` bytes of RAM at the absolute address `addr`.
    pub fn peek(&self, addr: u32, len: usize) -> Vec<u8> {
        let start = (addr - RAM_BASE) as usize;
        self.ram.lock().unwrap()[start..start + len].to_vec()
    }

    /// A copy of the log.
    pub fn log_lines(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Drop for FakeStub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

fn accept_loop(listener: TcpListener, shared: Shared) {
    let mut connections: Vec<JoinHandle<()>> = Vec::new();
    while !shared.stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let shared = shared.clone();
                connections.push(std::thread::spawn(move || serve(stream, shared)));
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => std::thread::sleep(POLL),
            Err(_) => std::thread::sleep(POLL),
        }
        connections.retain(|c| !c.is_finished());
    }
    for connection in connections {
        let _ = connection.join();
    }
}

/// One connection, from accept to disconnect.
fn serve(stream: TcpStream, shared: Shared) {
    if stream.set_nonblocking(false).is_err() || stream.set_read_timeout(Some(POLL)).is_err() {
        return;
    }
    let _ = stream.set_nodelay(true);
    // A client that stops reading must not block the stub's threads (and its drop) for good.
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    if shared.quirks.one_client && !wait_for_turn(&stream, &shared) {
        return;
    }
    shared.halted.store(true, Ordering::SeqCst);
    let mut conn = Conn {
        stream,
        shared: shared.clone(),
        buf: Vec::new(),
    };
    conn.run();
    // Like melonDS: without a client the emulation runs.
    shared.halted.store(false, Ordering::SeqCst);
    if shared.quirks.one_client {
        shared.serving.store(false, Ordering::SeqCst);
    }
}

/// Waits until no other client is served; `false` if this client left or the stub stops first.
fn wait_for_turn(stream: &TcpStream, shared: &Shared) -> bool {
    let mut probe = [0u8; 1];
    loop {
        if shared.stop.load(Ordering::SeqCst) {
            return false;
        }
        if shared
            .serving
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return true;
        }
        // Leave the bytes in the socket (served later), only notice a disconnect.
        match stream.peek(&mut probe) {
            Ok(0) => return false,
            Ok(_) => std::thread::sleep(POLL),
            Err(e) if is_timeout(&e) => {}
            Err(_) => return false,
        }
    }
}

fn is_timeout(e: &std::io::Error) -> bool {
    matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

fn checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0u8, |sum, b| sum.wrapping_add(*b))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// `addr,len` in hex.
fn parse_range(text: &str) -> Option<(u32, usize)> {
    let (addr, len) = text.split_once(',')?;
    Some((
        u32::from_str_radix(addr, 16).ok()?,
        usize::from_str_radix(len, 16).ok()?,
    ))
}

/// Offset into the RAM vector if `[addr, addr + len)` lies inside main RAM.
fn ram_offset(addr: u32, len: usize) -> Option<usize> {
    let start = addr.checked_sub(RAM_BASE)? as usize;
    (start.checked_add(len)? <= RAM_SIZE).then_some(start)
}

enum Step {
    Continue,
    Disconnect,
}

struct Conn {
    stream: TcpStream,
    shared: Shared,
    buf: Vec<u8>,
}

impl Conn {
    fn stopping(&self) -> bool {
        self.shared.stop.load(Ordering::SeqCst)
    }

    fn silent(&self) -> bool {
        !self.shared.quirks.answer || self.shared.wedged.load(Ordering::SeqCst)
    }

    fn log(&self, line: impl Into<String>) {
        self.shared.log.lock().unwrap().push(line.into());
    }

    /// Reads what is there into `buf`; `false` on disconnect, error or stop.
    fn fill(&mut self) -> bool {
        let mut chunk = [0u8; 4096];
        match self.stream.read(&mut chunk) {
            Ok(0) => false,
            Ok(n) => {
                self.buf.extend_from_slice(&chunk[..n]);
                true
            }
            Err(e) if is_timeout(&e) => !self.stopping(),
            Err(_) => false,
        }
    }

    fn send_raw(&mut self, bytes: &[u8]) -> bool {
        if self.silent() {
            return true;
        }
        self.stream.write_all(bytes).is_ok()
    }

    fn reply(&mut self, payload: &str) -> bool {
        let mut out = Vec::with_capacity(payload.len() + 4);
        out.push(b'$');
        out.extend_from_slice(payload.as_bytes());
        out.extend_from_slice(format!("#{:02x}", checksum(payload.as_bytes())).as_bytes());
        self.send_raw(&out)
    }

    fn run(&mut self) {
        if !self.handshake() {
            return;
        }
        while !self.stopping() {
            match self.process() {
                Some(Step::Disconnect) => return,
                Some(Step::Continue) => continue,
                None => {
                    if !self.fill() {
                        return;
                    }
                }
            }
        }
    }

    /// The first byte must be `+` within the ack deadline.
    fn handshake(&mut self) -> bool {
        let deadline = Instant::now() + self.shared.quirks.ack_deadline;
        while self.buf.is_empty() {
            if Instant::now() >= deadline || !self.fill() {
                return false;
            }
        }
        if self.buf[0] != b'+' {
            return false;
        }
        self.buf.remove(0);
        self.log("+");
        true
    }

    /// Handles one item from `buf`; `None` if more bytes are needed.
    fn process(&mut self) -> Option<Step> {
        // Acks and naks between packets.
        let skip = self
            .buf
            .iter()
            .take_while(|b| matches!(**b, b'+' | b'-'))
            .count();
        self.buf.drain(..skip);
        let first = *self.buf.first()?;
        if first == 0x03 {
            self.buf.remove(0);
            self.log("\x03");
            self.shared.halted.store(true, Ordering::SeqCst);
            return Some(self.answer("S05"));
        }
        if first != b'$' {
            // Garbage before a packet: drop it.
            self.buf.remove(0);
            return Some(Step::Continue);
        }
        let wedge_above = self.shared.quirks.wedge_above;
        let Some(hash) = self.buf.iter().position(|b| *b == b'#') else {
            if self.buf.len() > wedge_above {
                self.wedge();
            }
            return None;
        };
        if self.buf.len() < hash + 3 {
            return None;
        }
        let frame: Vec<u8> = self.buf.drain(..hash + 3).collect();
        if frame.len() > wedge_above {
            self.wedge();
            return Some(Step::Continue);
        }
        let payload = &frame[1..hash];
        let sum = std::str::from_utf8(&frame[hash + 1..])
            .ok()
            .and_then(|s| u8::from_str_radix(s, 16).ok());
        let text = String::from_utf8_lossy(payload).into_owned();
        self.log(text.clone());
        let no_ack = self.shared.no_ack.load(Ordering::SeqCst);
        if sum != Some(checksum(payload)) {
            if !no_ack && !self.send_raw(b"-") {
                return Some(Step::Disconnect);
            }
            return Some(Step::Continue);
        }
        if !no_ack && !self.send_raw(b"+") {
            return Some(Step::Disconnect);
        }
        Some(self.command(&text))
    }

    fn wedge(&mut self) {
        self.log("<wedged>");
        self.shared.wedged.store(true, Ordering::SeqCst);
        self.buf.clear();
    }

    fn answer(&mut self, payload: &str) -> Step {
        if self.reply(payload) {
            Step::Continue
        } else {
            Step::Disconnect
        }
    }

    fn command(&mut self, packet: &str) -> Step {
        if packet.starts_with("qSupported") {
            return self.answer(FEATURES);
        }
        if packet == "QStartNoAckMode" {
            let step = self.answer("OK");
            if !self.silent() {
                self.shared.no_ack.store(true, Ordering::SeqCst);
            }
            return step;
        }
        if packet == "?" {
            return self.answer("S02");
        }
        if packet == "vCont?" {
            return self.answer("vCont;c;s;t");
        }
        if packet == "c" || packet == "vCont;c" {
            self.shared.halted.store(false, Ordering::SeqCst);
            return Step::Continue;
        }
        if packet == "D" {
            self.answer("OK");
            return Step::Disconnect;
        }
        if let Some(range) = packet.strip_prefix('m') {
            return self.read_memory(range);
        }
        if let Some(rest) = packet.strip_prefix('M') {
            return self.write_memory(rest);
        }
        self.answer("")
    }

    fn read_memory(&mut self, range: &str) -> Step {
        let Some((addr, len)) = parse_range(range) else {
            return self.answer("E01");
        };
        let max_read = self.shared.quirks.max_read;
        if len == max_read + 1 {
            return Step::Continue;
        }
        if len > max_read {
            return self.answer("E02");
        }
        let Some(start) = ram_offset(addr, len) else {
            return self.answer("E01");
        };
        let hex = to_hex(&self.shared.ram.lock().unwrap()[start..start + len]);
        self.answer(&hex)
    }

    fn write_memory(&mut self, rest: &str) -> Step {
        let Some((range, hex)) = rest.split_once(':') else {
            return self.answer("E01");
        };
        let (Some((addr, len)), Some(data)) = (parse_range(range), from_hex(hex)) else {
            return self.answer("E01");
        };
        if data.len() != len {
            return self.answer("E01");
        }
        let Some(start) = ram_offset(addr, len) else {
            return self.answer("E01");
        };
        self.shared.ram.lock().unwrap()[start..start + len].copy_from_slice(&data);
        self.answer("OK")
    }
}
