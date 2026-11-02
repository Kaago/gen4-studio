//! RSP client for the ARM9 GDB stub of melonDS 1.1.
//!
//! The stub's quirks (see `docs/research/2026-10-30-phase-3d/probe-gdb-stub.md`): it halts the
//! emulation on connect until `c`; it closes a client that does not send `+` within 1 s; `m`
//! answers at most 575 bytes and an `M` packet above about 1150 characters wedges the stub until
//! melonDS restarts, so memory goes in [`CHUNK`]-sized packets; it serves a single client.
//! Memory reads and writes work while the emulation runs (one packet per frame).
//!
//! Every socket read and write has a timeout: [`REPLY_TIMEOUT`] per reply, the `timeout` of
//! [`GdbClient::connect`] during the handshake. After [`LiveError::NoReply`] the connection is in
//! an unknown state (a late reply may still arrive); callers drop the client and reconnect.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::flags::FlagKind;

/// Largest memory chunk per `m`/`M` packet (the stub answers `m` up to 575 bytes and wedges on
/// `M` packets above about 1150 characters).
pub const CHUNK: usize = 512;
/// How long a reply may take before the stub counts as not answering.
pub const REPLY_TIMEOUT: Duration = Duration::from_millis(1000);

/// Errors of the live connection; the German texts are shown in the studio as they are.
#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    #[error("cannot connect to {addr}: {source}")]
    Connect {
        addr: String,
        source: std::io::Error,
    },
    #[error("melonDS antwortet nicht")]
    NoReply,
    #[error("stub closed the connection")]
    Closed,
    #[error("stub answered {reply} to {packet}")]
    Rejected { packet: String, reply: String },
    #[error("melonDS spielt {found}, das Projekt ist {expected}")]
    WrongGame { found: String, expected: String },
    #[error("Nummer liegt außerhalb des Spielstands")]
    OutsideSave { kind: FlagKind, number: u16 },
    #[error("noch kein Spielstand geladen")]
    NoSave,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// The RSP checksum: the sum of the payload bytes modulo 256.
pub(crate) fn checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0u8, |sum, b| sum.wrapping_add(*b))
}

/// `$payload#xx` with the checksum in lower-case hex.
pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.push(b'$');
    out.extend_from_slice(payload);
    out.extend_from_slice(format!("#{:02x}", checksum(payload)).as_bytes());
    out
}

/// The interrupt byte, sent without a frame.
const INTERRUPT: u8 = 0x03;

/// Maps a socket error to the live error: timeouts are [`LiveError::NoReply`], a reset or broken
/// pipe is [`LiveError::Closed`].
fn socket_error(e: std::io::Error) -> LiveError {
    match e.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => LiveError::NoReply,
        ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted
        | ErrorKind::BrokenPipe
        | ErrorKind::UnexpectedEof
        | ErrorKind::NotConnected => LiveError::Closed,
        _ => LiveError::Io(e),
    }
}

/// `E` plus two hex digits.
fn is_error_reply(reply: &str) -> bool {
    reply.len() == 3 && reply.starts_with('E') && reply[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// `S`/`T` plus two hex digits (`S02`, `S05`): the stub is halted.
fn is_stop_reply(reply: &str) -> bool {
    reply.len() >= 3
        && (reply.starts_with('S') || reply.starts_with('T'))
        && reply.as_bytes()[1..3].iter().all(u8::is_ascii_hexdigit)
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// The address of chunk `offset` after `addr`; an error if it runs past the 32-bit space.
fn chunk_addr(addr: u32, offset: usize, len: usize) -> Result<u32, LiveError> {
    u32::try_from(offset)
        .ok()
        .and_then(|o| addr.checked_add(o))
        .filter(|a| u32::try_from(len).is_ok_and(|l| a.checked_add(l).is_some()))
        .ok_or_else(|| {
            LiveError::Io(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!("memory range {addr:#x}+{offset:#x} runs past 4 GiB"),
            ))
        })
}

/// A connection to the stub. Created halted (the stub halts on connect); [`GdbClient::resume`]
/// lets the game run.
#[derive(Debug)]
pub struct GdbClient {
    stream: TcpStream,
    /// Received bytes not yet consumed.
    buf: Vec<u8>,
    no_ack: bool,
    halted: bool,
    features: String,
    reply_timeout: Duration,
}

impl GdbClient {
    /// Connects, sends `+`, `qSupported` and `QStartNoAckMode`. The stub is halted afterwards
    /// (no `c` is sent). `timeout` bounds the TCP connect and every handshake reply.
    pub fn connect(addr: &str, timeout: Duration) -> Result<GdbClient, LiveError> {
        let timeout = timeout.max(Duration::from_millis(1));
        let connect_error = |source| LiveError::Connect {
            addr: addr.to_string(),
            source,
        };
        let targets: Vec<SocketAddr> = addr.to_socket_addrs().map_err(connect_error)?.collect();
        let mut last = std::io::Error::new(ErrorKind::AddrNotAvailable, "no address");
        let mut stream = None;
        for target in targets {
            match TcpStream::connect_timeout(&target, timeout) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(e) => last = e,
            }
        }
        let stream = stream.ok_or_else(|| connect_error(last))?;
        stream.set_nodelay(true).map_err(connect_error)?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(connect_error)?;
        let mut client = GdbClient {
            stream,
            buf: Vec::new(),
            no_ack: false,
            halted: true,
            features: String::new(),
            reply_timeout: timeout,
        };
        // The stub closes the connection without a `+` within 1 s.
        client.send_raw(b"+")?;
        client.features = client.command("qSupported")?;
        if client.features.contains("QStartNoAckMode+") {
            let reply = client.command("QStartNoAckMode")?;
            if reply != "OK" {
                return Err(LiveError::Rejected {
                    packet: "QStartNoAckMode".into(),
                    reply,
                });
            }
            client.no_ack = true;
        }
        client.reply_timeout = REPLY_TIMEOUT;
        client
            .stream
            .set_write_timeout(Some(REPLY_TIMEOUT))
            .map_err(LiveError::Io)?;
        Ok(client)
    }

    /// The `qSupported` reply.
    pub fn features(&self) -> &str {
        &self.features
    }

    /// Whether the stub is halted (after connect and [`GdbClient::halt`]).
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Halts the emulation (`\x03`) and waits for the stop reply; a no-op while halted.
    pub fn halt(&mut self) -> Result<(), LiveError> {
        if self.halted {
            return Ok(());
        }
        self.send_raw(&[INTERRUPT])?;
        let reply = self.read_packet()?;
        if !is_stop_reply(&reply) {
            return Err(LiveError::Rejected {
                packet: "\\x03".into(),
                reply,
            });
        }
        self.halted = true;
        Ok(())
    }

    /// Lets the emulation run (`c`, no reply); a no-op while running.
    pub fn resume(&mut self) -> Result<(), LiveError> {
        if !self.halted {
            return Ok(());
        }
        self.send_packet(b"c")?;
        self.halted = false;
        Ok(())
    }

    /// Reads `len` bytes at `addr` in packets of at most [`CHUNK`] bytes.
    pub fn read(&mut self, addr: u32, len: usize) -> Result<Vec<u8>, LiveError> {
        let mut out = Vec::with_capacity(len);
        let mut offset = 0;
        while offset < len {
            let n = CHUNK.min(len - offset);
            let at = chunk_addr(addr, offset, n)?;
            let packet = format!("m{at:08x},{n:x}");
            let reply = self.command(&packet)?;
            match decode_hex(&reply) {
                Some(bytes) if bytes.len() == n && !is_error_reply(&reply) => {
                    out.extend_from_slice(&bytes)
                }
                _ => return Err(LiveError::Rejected { packet, reply }),
            }
            offset += n;
        }
        Ok(out)
    }

    /// Writes `data` at `addr` in packets of at most [`CHUNK`] bytes, each answered with `OK`.
    pub fn write(&mut self, addr: u32, data: &[u8]) -> Result<(), LiveError> {
        for (i, chunk) in data.chunks(CHUNK).enumerate() {
            let at = chunk_addr(addr, i * CHUNK, chunk.len())?;
            let head = format!("M{at:08x},{:x}", chunk.len());
            let packet = format!("{head}:{}", encode_hex(chunk));
            let reply = self.command(&packet)?;
            if reply != "OK" {
                return Err(LiveError::Rejected {
                    packet: head,
                    reply,
                });
            }
        }
        Ok(())
    }

    /// Detaches (`D`); melonDS answers `OK` and lets the emulation run.
    pub fn detach(mut self) -> Result<(), LiveError> {
        let reply = self.command("D")?;
        if reply != "OK" {
            return Err(LiveError::Rejected {
                packet: "D".into(),
                reply,
            });
        }
        self.halted = false;
        Ok(())
    }

    fn send_raw(&mut self, bytes: &[u8]) -> Result<(), LiveError> {
        self.stream.write_all(bytes).map_err(socket_error)
    }

    fn send_packet(&mut self, payload: &[u8]) -> Result<(), LiveError> {
        self.send_raw(&frame(payload))
    }

    /// Sends a packet and returns its reply. A stop reply that arrives while the client thinks
    /// the stub runs (it stopped on its own) marks it halted and is skipped; replies to `m`, `M`
    /// and the handshake never start with `S`/`T`.
    fn command(&mut self, payload: &str) -> Result<String, LiveError> {
        self.send_packet(payload.as_bytes())?;
        loop {
            let reply = self.read_packet()?;
            if is_stop_reply(&reply) && !self.halted {
                self.halted = true;
                continue;
            }
            return Ok(reply);
        }
    }

    /// Reads the next packet within the reply timeout, verifies its checksum and acks it unless
    /// in no-ack mode. Acks of the stub are skipped.
    fn read_packet(&mut self) -> Result<String, LiveError> {
        let deadline = Instant::now() + self.reply_timeout;
        loop {
            if let Some(payload) = self.take_packet()? {
                return Ok(payload);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(LiveError::NoReply);
            }
            self.fill(left)?;
        }
    }

    /// One complete packet from `buf`, if there is one.
    fn take_packet(&mut self) -> Result<Option<String>, LiveError> {
        loop {
            // Acks, naks and stray bytes before a packet.
            let skip = self.buf.iter().take_while(|b| **b != b'$').count();
            self.buf.drain(..skip);
            let Some(hash) = self.buf.iter().position(|b| *b == b'#') else {
                return Ok(None);
            };
            if self.buf.len() < hash + 3 {
                return Ok(None);
            }
            let packet: Vec<u8> = self.buf.drain(..hash + 3).collect();
            let payload = &packet[1..hash];
            let sum = std::str::from_utf8(&packet[hash + 1..])
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok());
            if sum != Some(checksum(payload)) {
                // Ask for a resend; in no-ack mode the stub does not resend, so wait on.
                if !self.no_ack {
                    self.send_raw(b"-")?;
                }
                continue;
            }
            if !self.no_ack {
                self.send_raw(b"+")?;
            }
            return Ok(Some(String::from_utf8_lossy(payload).into_owned()));
        }
    }

    fn fill(&mut self, timeout: Duration) -> Result<(), LiveError> {
        self.stream
            .set_read_timeout(Some(timeout.max(Duration::from_millis(1))))
            .map_err(socket_error)?;
        let mut chunk = [0u8; 4096];
        match self.stream.read(&mut chunk) {
            Ok(0) => Err(LiveError::Closed),
            Ok(n) => {
                self.buf.extend_from_slice(&chunk[..n]);
                Ok(())
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
            Err(e) => Err(socket_error(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::fake_stub::{FakeStub, Quirks};
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    const SAVE: u32 = 0x0227_E354;

    fn strict() -> bool {
        std::env::var("G4_TIMING_STRICT").is_ok_and(|v| v == "1")
    }

    fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !cond() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn packets(stub: &FakeStub, prefix: char) -> Vec<String> {
        stub.log_lines()
            .into_iter()
            .filter(|l| l.starts_with(prefix))
            .collect()
    }

    /// The length field (hex) of an `m`/`M` log line.
    fn packet_len(line: &str) -> usize {
        let range = line[1..].split(':').next().unwrap();
        usize::from_str_radix(range.split(',').nth(1).unwrap(), 16).unwrap()
    }

    fn connected(quirks: Quirks) -> (FakeStub, GdbClient) {
        let stub = FakeStub::start(quirks);
        let started = Instant::now();
        let client = GdbClient::connect(&stub.addr(), REPLY_TIMEOUT).expect("connect");
        let took = started.elapsed();
        println!("connect took {took:?}");
        if strict() {
            assert!(took <= Duration::from_millis(100), "connect took {took:?}");
        }
        (stub, client)
    }

    #[test]
    fn checksum_and_frame_match_the_rsp_examples() {
        assert_eq!(checksum(b"OK"), 0x9A);
        assert_eq!(frame(b"m02101EE0,4"), b"$m02101EE0,4#7b".to_vec());
        assert_eq!(frame(b""), b"$#00".to_vec());
    }

    #[test]
    fn connect_acks_first_and_reads_the_features() {
        let (stub, client) = connected(Quirks::melonds_1_1());
        assert!(client.features().contains("PacketSize=47F"), "{}", client.features());
        let log = stub.log_lines();
        assert_eq!(log[0], "+");
        assert!(log[1].starts_with("qSupported"), "{log:?}");
        assert_eq!(log[2], "QStartNoAckMode");
        assert!(client.is_halted());
        assert!(stub.halted.load(Ordering::SeqCst));
        assert!(!log.iter().any(|l| l == "c"), "connect must not resume: {log:?}");
    }

    #[test]
    fn read_splits_at_512_bytes_and_joins_the_reply() {
        let (stub, mut client) = connected(Quirks::melonds_1_1());
        let data: Vec<u8> = (0..940u32).map(|i| (i * 7 + 3) as u8).collect();
        stub.poke(SAVE, &data);
        assert_eq!(client.read(SAVE, 940).unwrap(), data);
        let reads = packets(&stub, 'm');
        assert_eq!(reads.len(), 2, "{reads:?}");
        assert_eq!(packet_len(&reads[0]), 512);
        assert_eq!(packet_len(&reads[1]), 428);
        assert_eq!(reads[0], "m0227e354,200");
        assert_eq!(reads[1], "m0227e554,1ac");
    }

    #[test]
    fn write_splits_at_512_bytes_and_never_wedges_the_stub() {
        let (stub, mut client) = connected(Quirks::melonds_1_1());
        let data: Vec<u8> = (0..1100u32).map(|i| (i * 13 + 1) as u8).collect();
        client.write(SAVE, &data).unwrap();
        let writes = packets(&stub, 'M');
        assert_eq!(writes.len(), 3);
        assert!(writes.iter().all(|w| packet_len(w) <= CHUNK), "{writes:?}");
        assert_eq!(writes.iter().map(|w| packet_len(w)).sum::<usize>(), 1100);
        assert!(!stub.wedged.load(Ordering::SeqCst));
        assert_eq!(stub.peek(SAVE, 1100), data);
        assert_eq!(client.read(SAVE, 1100).unwrap(), data);
    }

    #[test]
    fn halt_waits_for_the_stop_reply_and_resume_sends_c() {
        let (stub, mut client) = connected(Quirks::melonds_1_1());
        client.resume().unwrap();
        assert!(!client.is_halted());
        wait_until("the stub to run", || !stub.halted.load(Ordering::SeqCst));
        client.resume().unwrap();
        // A reply proves the stub has handled everything sent before it.
        client.read(SAVE, 4).unwrap();
        let resumes = stub.log_lines().iter().filter(|l| *l == "c").count();
        assert_eq!(resumes, 1, "second resume is a no-op");

        client.halt().unwrap();
        assert!(client.is_halted());
        assert!(stub.halted.load(Ordering::SeqCst));
        assert!(stub.log_lines().iter().any(|l| l == "\x03"));
        let mut expected = stub.log_lines();
        client.halt().unwrap();
        // The stop reply was consumed: the next reply belongs to the next packet.
        stub.poke(SAVE, &[1, 2, 3, 4]);
        assert_eq!(client.read(SAVE, 4).unwrap(), vec![1, 2, 3, 4]);
        expected.push("m0227e354,4".into());
        assert_eq!(stub.log_lines(), expected, "second halt sends nothing");
    }

    #[test]
    fn a_stub_without_reply_fails_with_no_reply_within_the_timeout() {
        let stub = FakeStub::start(Quirks {
            answer: false,
            ..Quirks::melonds_1_1()
        });
        let started = Instant::now();
        let result = GdbClient::connect(&stub.addr(), Duration::from_millis(200));
        let took = started.elapsed();
        println!("no reply after {took:?}");
        assert!(matches!(result, Err(LiveError::NoReply)), "{result:?}");
        assert!(took >= Duration::from_millis(200), "{took:?}");
        let bound = if strict() { 500 } else { 5000 };
        assert!(took < Duration::from_millis(bound), "{took:?}");
    }

    #[test]
    fn read_of_600_bytes_is_two_packets_of_512_and_88() {
        let (stub, mut client) = connected(Quirks::melonds_1_1());
        assert_eq!(client.read(SAVE, 600).unwrap().len(), 600);
        let reads = packets(&stub, 'm');
        assert_eq!(reads.len(), 2);
        assert!(reads[0].ends_with(",200"), "{reads:?}");
        assert!(reads[1].ends_with(",58"), "{reads:?}");
    }

    #[test]
    fn a_rejected_write_is_an_error_with_the_reply() {
        let (_stub, mut client) = connected(Quirks::melonds_1_1());
        match client.write(0x0100_0000, &[1]) {
            Err(LiveError::Rejected { packet, reply }) => {
                assert_eq!(reply, "E01");
                assert!(packet.starts_with("M01000000,1"), "{packet}");
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
        let result = client.read(0x0100_0000, 4);
        assert!(
            matches!(&result, Err(LiveError::Rejected { reply, .. }) if reply == "E01"),
            "{result:?}"
        );
    }

    #[test]
    fn detach_sends_d_and_lets_the_emulation_run() {
        let (stub, client) = connected(Quirks::melonds_1_1());
        client.detach().unwrap();
        assert!(stub.log_lines().iter().any(|l| l == "D"));
        wait_until("the stub to run", || !stub.halted.load(Ordering::SeqCst));
        // melonDS serves the next client after a detach.
        let again = GdbClient::connect(&stub.addr(), REPLY_TIMEOUT).unwrap();
        assert!(again.is_halted());
    }

    #[test]
    fn a_dropped_stub_closes_the_connection() {
        let (stub, mut client) = connected(Quirks::melonds_1_1());
        drop(stub);
        let result = client.read(SAVE, 4);
        assert!(matches!(result, Err(LiveError::Closed)), "{result:?}");
    }

    #[test]
    fn nothing_listening_is_a_connect_error() {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let addr = format!("127.0.0.1:{port}");
        let result = GdbClient::connect(&addr, Duration::from_millis(200));
        assert!(
            matches!(&result, Err(LiveError::Connect { addr: a, .. }) if *a == addr),
            "{result:?}"
        );
    }

    #[test]
    fn empty_reads_and_writes_send_nothing() {
        let (stub, mut client) = connected(Quirks::melonds_1_1());
        assert!(client.read(SAVE, 0).unwrap().is_empty());
        client.write(SAVE, &[]).unwrap();
        assert!(packets(&stub, 'm').is_empty());
        assert!(packets(&stub, 'M').is_empty());
    }

    // The fake itself must behave like melonDS 1.1, or the client tests prove nothing.

    fn raw(stub: &FakeStub) -> std::net::TcpStream {
        let stream = std::net::TcpStream::connect(stub.addr()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        stream
    }

    /// Everything the stub sends within the read timeout (empty on silence, `None` on EOF).
    fn drain(stream: &mut std::net::TcpStream) -> Option<Vec<u8>> {
        use std::io::Read;
        let mut out = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) if out.is_empty() => return None,
                Ok(0) => return Some(out),
                Ok(n) => out.extend_from_slice(&chunk[..n]),
                Err(_) => return Some(out),
            }
        }
    }

    fn send(stream: &mut std::net::TcpStream, bytes: &[u8]) {
        use std::io::Write;
        stream.write_all(bytes).unwrap();
    }

    #[test]
    fn fake_drops_a_client_without_the_initial_ack() {
        let stub = FakeStub::start(Quirks {
            ack_deadline: Duration::from_millis(100),
            ..Quirks::melonds_1_1()
        });
        let mut stream = raw(&stub);
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(drain(&mut stream), None, "the stub must have closed");
    }

    #[test]
    fn fake_reads_up_to_575_bytes_ignores_576_and_rejects_577() {
        let stub = FakeStub::start(Quirks::melonds_1_1());
        let mut stream = raw(&stub);
        send(&mut stream, b"+");
        send(&mut stream, &frame(b"m02000000,23f"));
        let reply = drain(&mut stream).unwrap();
        assert_eq!(reply.len(), 1 + 1 + 575 * 2 + 3, "ack + $hex#xx");
        send(&mut stream, &frame(b"m02000000,240"));
        assert_eq!(drain(&mut stream).unwrap(), b"+".to_vec(), "576: no reply");
        send(&mut stream, &frame(b"m02000000,241"));
        assert_eq!(drain(&mut stream).unwrap(), b"+$E02#a7".to_vec());
    }

    #[test]
    fn fake_wedges_for_good_on_a_too_long_packet() {
        let stub = FakeStub::start(Quirks::melonds_1_1());
        let mut stream = raw(&stub);
        send(&mut stream, b"+");
        let mut packet = b"M02000000,23f:".to_vec();
        packet.extend(std::iter::repeat_n(b'0', 575 * 2));
        send(&mut stream, &frame(&packet));
        assert_eq!(drain(&mut stream).unwrap(), b"".to_vec());
        assert!(stub.wedged.load(Ordering::SeqCst));
        send(&mut stream, &frame(b"m02000000,4"));
        assert_eq!(drain(&mut stream).unwrap(), b"".to_vec(), "no reply ever again");
        drop(stream);
        let result = GdbClient::connect(&stub.addr(), Duration::from_millis(200));
        assert!(matches!(result, Err(LiveError::NoReply)), "{result:?}");
    }

    #[test]
    fn fake_serves_a_second_client_only_after_the_first_left() {
        let stub = FakeStub::start(Quirks::melonds_1_1());
        let mut first = GdbClient::connect(&stub.addr(), REPLY_TIMEOUT).unwrap();
        let second = GdbClient::connect(&stub.addr(), Duration::from_millis(200));
        assert!(matches!(second, Err(LiveError::NoReply)), "{second:?}");
        // The first client keeps working.
        assert_eq!(first.read(SAVE, 4).unwrap(), vec![0; 4]);
        first.detach().unwrap();
        let third = GdbClient::connect(&stub.addr(), REPLY_TIMEOUT).unwrap();
        assert!(third.features().contains("QStartNoAckMode+"));
    }

    #[test]
    fn fake_forgets_a_queued_client_that_left_before_its_turn() {
        let stub = FakeStub::start(Quirks::melonds_1_1());
        let first = GdbClient::connect(&stub.addr(), REPLY_TIMEOUT).unwrap();
        // The queued client sends `+` and `qSupported`, gets no reply and leaves.
        let queued = GdbClient::connect(&stub.addr(), Duration::from_millis(200));
        assert!(matches!(queued, Err(LiveError::NoReply)), "{queued:?}");
        drop(queued);
        // Its thread ends although its bytes still sit unread ahead of the FIN.
        wait_until("the queued client's thread to end", || {
            stub.open_connections() == 1
        });
        first.detach().unwrap();
        let third = GdbClient::connect(&stub.addr(), REPLY_TIMEOUT).unwrap();
        third.detach().unwrap();
        wait_until("all threads to end", || stub.open_connections() == 0);
        let log = stub.log_lines();
        let handshakes = log.iter().filter(|l| l.starts_with("qSupported")).count();
        assert_eq!(handshakes, 2, "only the first and the third client: {log:?}");
        assert_eq!(log.iter().filter(|l| *l == "+").count(), 2, "{log:?}");
        assert!(!stub.halted.load(Ordering::SeqCst));
    }
}
