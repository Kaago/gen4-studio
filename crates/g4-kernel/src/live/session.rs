//! A live session with the running game: consistent snapshots, writes with read-back and
//! watches that halt.

use std::io::ErrorKind;
use std::time::{Duration, Instant};

use crate::flags::{SaveBlock, save_block};

use super::gdb::{GdbClient, LiveError};
use super::save::{GAME_CODE_ADDR, SaveView, Snapshot, Target, in_block};

/// A watched flag or variable; a change halts the game.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Watch {
    pub id: u32,
    pub target: Target,
    pub label: String,
}

/// The game was halted because a watched value changed.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Halt {
    pub watch: Watch,
    pub before: u16,
    pub after: u16,
    /// The revision of the snapshot that saw the change.
    pub revision: u64,
}

/// Whether the game code in RAM was compared with the project's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameCheck {
    Verified,
    Unverified,
}

/// A connection to the stub of the running game.
#[derive(Debug)]
pub struct LiveSession {
    client: GdbClient,
    game_code: String,
    block: SaveBlock,
    view: Option<SaveView>,
    last: Option<Snapshot>,
    watches: Vec<Watch>,
    halt: Option<Halt>,
    next_watch_id: u32,
    pending_write: Option<(Target, u16)>,
    read_back: Option<bool>,
    revision: u64,
}

impl LiveSession {
    /// Connects (the stub halts the game), checks the game code at [`GAME_CODE_ADDR`], reads the
    /// save pointer and lets the game run (`c`).
    ///
    /// - A game code of four ASCII letters or digits that differs from `game_code` is
    ///   [`LiveError::WrongGame`]: nothing is written, the session detaches (`D`, the game runs).
    ///   Any other four bytes give [`GameCheck::Unverified`].
    /// - An invalid pointer (the save is not loaded yet) is no error: the session exists with
    ///   [`LiveSession::has_save`] `false`, and every [`LiveSession::refresh`] looks again.
    /// - A `game_code` without a measured save layout fails before connecting.
    ///
    /// `timeout` bounds the TCP connect and each handshake reply (a stub that accepts but never
    /// answers gives [`LiveError::NoReply`] after it); later replies get [`REPLY_TIMEOUT`].
    ///
    /// [`REPLY_TIMEOUT`]: super::gdb::REPLY_TIMEOUT
    pub fn connect(
        addr: &str,
        game_code: &str,
        timeout: Duration,
    ) -> Result<(LiveSession, GameCheck), LiveError> {
        let block = save_block(game_code).ok_or_else(|| {
            LiveError::Io(std::io::Error::new(
                ErrorKind::Unsupported,
                format!("Für {game_code} ist die Lage des Spielstands nicht bekannt"),
            ))
        })?;
        // From here on the stub is halted; on any error the client is dropped, which closes the
        // connection, and melonDS lets the game run.
        let mut client = GdbClient::connect(addr, timeout)?;
        let code = client.read(GAME_CODE_ADDR, 4)?;
        let check = if code.iter().all(u8::is_ascii_alphanumeric) {
            let found = String::from_utf8_lossy(&code).into_owned();
            if found != game_code {
                let _ = client.detach();
                return Err(LiveError::WrongGame {
                    found,
                    expected: game_code.to_string(),
                });
            }
            GameCheck::Verified
        } else {
            GameCheck::Unverified
        };
        let mut session = LiveSession {
            client,
            game_code: game_code.to_string(),
            block,
            view: None,
            last: None,
            watches: Vec::new(),
            halt: None,
            next_watch_id: 1,
            pending_write: None,
            read_back: None,
            revision: 0,
        };
        match session.locate() {
            Ok(_) | Err(LiveError::NoSave) => {}
            Err(e) => return Err(e),
        }
        session.client.resume()?;
        Ok((session, check))
    }

    /// The game code of the project this session was opened for.
    pub fn game_code(&self) -> &str {
        &self.game_code
    }

    /// Whether the save pointer was valid at the last look.
    pub fn has_save(&self) -> bool {
        self.view.is_some()
    }

    /// One consistent snapshot: halt → pointer → vars → flags → continue, all within one halt.
    ///
    /// The pointer is read on every cycle (4 bytes): it detects the save being loaded and a reset
    /// back to the title. While it is invalid, the view and [`LiveSession::last`] are dropped,
    /// the game continues and the result is [`LiveError::NoSave`]: not fatal, the session stays
    /// usable and the next refresh looks again. When the block moved (another start), the old
    /// snapshot is dropped too, so no watch compares values of two different saves.
    ///
    /// The values are compared with the last snapshot: a changed watched value sets
    /// [`LiveSession::halt`] (the first watch that changed) and the game stays halted, unless the
    /// change is our own pending write with exactly the written value. While halted by a watch,
    /// refresh reads but never continues; [`LiveSession::resume`] does.
    ///
    /// Any other error ([`LiveError::NoReply`], [`LiveError::Closed`], …) leaves the connection in
    /// an unknown state: the caller drops the session (closing the socket lets the game run) and
    /// reconnects. The session never panics on it.
    pub fn refresh(&mut self) -> Result<&Snapshot, LiveError> {
        let started = Instant::now();
        self.client.halt()?;
        let snapshot = match self.read_snapshot(started) {
            Ok(snapshot) => snapshot,
            Err(e) => {
                self.continue_unless_watched();
                return Err(e);
            }
        };
        let own = self.pending_write.take();
        if let Some((target, value)) = own {
            self.read_back = Some(snapshot.value(target) == Some(value));
        }
        if self.halt.is_none()
            && let Some(last) = &self.last
        {
            self.halt = self.watches.iter().find_map(|watch| {
                let before = last.value(watch.target)?;
                let after = snapshot.value(watch.target)?;
                let changed = before != after && own != Some((watch.target, after));
                changed.then(|| Halt {
                    watch: watch.clone(),
                    before,
                    after,
                    revision: snapshot.revision,
                })
            });
        }
        if self.halt.is_none() {
            self.client.resume()?;
        }
        Ok(self.last.insert(snapshot))
    }

    /// The last snapshot; `None` before the first and while there is no save.
    pub fn last(&self) -> Option<&Snapshot> {
        self.last.as_ref()
    }

    /// Writes a flag (any value but 0 sets it) or a variable: halt → pointer → (read the byte →
    /// change the bit, for a flag) → write → continue. While halted by a watch the game stays
    /// halted. Whether the value held is decided by the next [`LiveSession::refresh`]
    /// ([`LiveSession::read_back`]).
    ///
    /// Numbers outside the save are [`LiveError::OutsideSave`] (checked first, from the layout);
    /// without a save it is [`LiveError::NoSave`]. Nothing is written in either case.
    pub fn set(&mut self, target: Target, value: u16) -> Result<(), LiveError> {
        if !in_block(&self.block, target) {
            return Err(target.outside_save());
        }
        if self.view.is_none() {
            return Err(LiveError::NoSave);
        }
        let value = match target {
            Target::Flag { .. } => u16::from(value != 0),
            Target::Var { .. } => value,
        };
        self.client.halt()?;
        if let Err(e) = self.write(target, value) {
            self.continue_unless_watched();
            return Err(e);
        }
        self.pending_write = Some((target, value));
        self.read_back = None;
        if self.halt.is_none() {
            self.client.resume()?;
        }
        Ok(())
    }

    /// After a [`LiveSession::set`]: `None` until the next refresh, then `Some(true)` when the
    /// snapshot showed the written value, `Some(false)` when not (or the save was gone). Stays
    /// until the next `set`.
    pub fn read_back(&self) -> Option<bool> {
        self.read_back
    }

    /// Watches a flag or variable; works without a save (the range comes from the layout).
    /// Numbers outside the save are [`LiveError::OutsideSave`].
    pub fn add_watch(&mut self, target: Target, label: String) -> Result<Watch, LiveError> {
        if !in_block(&self.block, target) {
            return Err(target.outside_save());
        }
        let watch = Watch {
            id: self.next_watch_id,
            target,
            label,
        };
        self.next_watch_id += 1;
        self.watches.push(watch.clone());
        Ok(watch)
    }

    /// Removes a watch; `false` for an unknown id. A halt it caused stays until resumed.
    pub fn remove_watch(&mut self, id: u32) -> bool {
        let before = self.watches.len();
        self.watches.retain(|w| w.id != id);
        self.watches.len() != before
    }

    pub fn watches(&self) -> &[Watch] {
        &self.watches
    }

    /// The watch halt the game stands in, if any.
    pub fn halt(&self) -> Option<&Halt> {
        self.halt.as_ref()
    }

    /// Clears the halt and lets the game run (`c`).
    pub fn resume(&mut self) -> Result<(), LiveError> {
        self.halt = None;
        self.client.resume()
    }

    /// Detaches (`D`): melonDS lets the game run, also out of a watch halt.
    pub fn detach(self) -> Result<(), LiveError> {
        self.client.detach()
    }

    /// Reads the pointer (the stub is halted) and updates the view. An invalid pointer drops the
    /// view and the last snapshot, decides a pending write as not held and is
    /// [`LiveError::NoSave`]; a moved block drops the last snapshot.
    fn locate(&mut self) -> Result<SaveView, LiveError> {
        let bytes = self.client.read(self.block.pointer, 4)?;
        let mut word = [0u8; 4];
        word.copy_from_slice(&bytes);
        let view = SaveView::resolve(self.block, u32::from_le_bytes(word));
        if view.map(|v| v.base) != self.view.map(|v| v.base) {
            self.last = None;
        }
        self.view = view;
        match view {
            Some(view) => Ok(view),
            None => {
                if self.pending_write.take().is_some() {
                    self.read_back = Some(false);
                }
                Err(LiveError::NoSave)
            }
        }
    }

    /// Pointer, vars and flags (the stub is halted); counts the revision.
    fn read_snapshot(&mut self, started: Instant) -> Result<Snapshot, LiveError> {
        let view = self.locate()?;
        let (vars_addr, vars_len) = view.vars_range();
        let vars = self.client.read(vars_addr, vars_len)?;
        let (flags_addr, flags_len) = view.flags_range();
        let flags = self.client.read(flags_addr, flags_len)?;
        self.revision += 1;
        Ok(Snapshot {
            revision: self.revision,
            flags,
            vars: vars
                .as_chunks::<2>()
                .0
                .iter()
                .map(|w| u16::from_le_bytes(*w))
                .collect(),
            taken_ms: u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX),
        })
    }

    /// Writes `value` (a flag already 0 or 1) at the target's address (the stub is halted).
    fn write(&mut self, target: Target, value: u16) -> Result<(), LiveError> {
        let view = self.locate()?;
        match target {
            Target::Flag { number } => {
                let (addr, bit) = view
                    .flag_addr(number)
                    .ok_or_else(|| target.outside_save())?;
                // `read` returns exactly the requested length or an error.
                let byte = self.client.read(addr, 1)?[0];
                let byte = if value != 0 {
                    byte | (1 << bit)
                } else {
                    byte & !(1 << bit)
                };
                self.client.write(addr, &[byte])
            }
            Target::Var { number } => {
                let addr = view
                    .var_addr(number)
                    .ok_or_else(|| target.outside_save())?;
                self.client.write(addr, &value.to_le_bytes())
            }
        }
    }

    /// After a failed cycle: let the game run again unless a watch holds it. Best effort, the
    /// first error is the one reported.
    fn continue_unless_watched(&mut self) {
        if self.halt.is_none() {
            let _ = self.client.resume();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::FlagKind;
    use crate::live::fake_stub::{FakeStub, Quirks};
    use crate::live::{CHUNK, REPLY_TIMEOUT};
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    /// Platinum: the u32 at `POINTER` points at `BLOCK`; the layout counts from `BASE`.
    const POINTER: u32 = 0x0210_1EE0;
    const BLOCK: u32 = 0x0227_E354;
    const BASE: u32 = BLOCK + 0x14;
    const FLAGS: u32 = BASE + 0xFEC;
    const VARS: u32 = BASE + 0xDAC;
    /// Flag 0x173 is bit 3 of byte 0x2E.
    const FLAG_173_BYTE: u32 = FLAGS + 0x2E;
    const VAR_40A5: u32 = VARS + 2 * 0xA5;

    const FLAG_173: Target = Target::Flag { number: 0x173 };
    const VAR_A5: Target = Target::Var { number: 0x40A5 };

    fn strict() -> bool {
        std::env::var("G4_TIMING_STRICT").is_ok_and(|v| v == "1")
    }

    /// Upper bound of a timing assert: `ms` under `G4_TIMING_STRICT=1`, generous otherwise.
    fn bound(ms: u64) -> Duration {
        Duration::from_millis(if strict() { ms } else { 5000 })
    }

    fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !cond() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn runs(stub: &FakeStub) {
        wait_until("the stub to run", || !stub.halted.load(Ordering::SeqCst));
    }

    /// A fake with the Platinum game code and the pointer to the save block.
    fn platinum(quirks: Quirks) -> FakeStub {
        let stub = FakeStub::start(quirks);
        stub.poke(GAME_CODE_ADDR, b"CPUD");
        stub.poke(POINTER, &BLOCK.to_le_bytes());
        stub
    }

    fn session(stub: &FakeStub) -> LiveSession {
        let (session, check) =
            LiveSession::connect(&stub.addr(), "CPUD", REPLY_TIMEOUT).expect("connect");
        assert_eq!(check, GameCheck::Verified);
        session
    }

    fn count(stub: &FakeStub, line: &str) -> usize {
        stub.log_lines().iter().filter(|l| *l == line).count()
    }

    #[test]
    fn save_view_resolves_platinum_addresses_from_the_metadata() {
        let view = SaveView::resolve(save_block("CPUD").unwrap(), 0x0227_E354).unwrap();
        assert_eq!(view.base, 0x0227_E354 + 0x14);
        assert_eq!(
            view.flag_addr(0x173),
            Some((0x0227_E354 + 0x14 + 0xFEC + 0x2E, 3))
        );
        assert_eq!(
            view.var_addr(0x40A5),
            Some(0x0227_E354 + 0x14 + 0xDAC + 2 * 0xA5)
        );
        assert_eq!(view.flag_addr(364 * 8), None);
        assert_eq!(view.flag_addr(364 * 8 - 1), Some((FLAGS + 363, 7)));
        assert_eq!(view.var_addr(0x4000 + 288), None);
        assert_eq!(view.var_addr(0x4000 + 287), Some(VARS + 2 * 287));
        assert_eq!(view.var_addr(0x3FFF), None);
        assert_eq!(view.flags_range(), (FLAGS, 364));
        assert_eq!(view.vars_range(), (VARS, 576));
        assert_eq!(SaveView::resolve(save_block("CPUD").unwrap(), 0), None);
        // Outside RAM, not aligned, or the block would run past the end of RAM.
        for pointer in [0x0100_0000, 0x0240_0000, 0x0227_E355, 0x023F_F000] {
            assert_eq!(
                SaveView::resolve(save_block("CPUD").unwrap(), pointer),
                None,
                "{pointer:#x}"
            );
        }
    }

    #[test]
    fn snapshot_values_and_target_json() {
        let snapshot = Snapshot {
            revision: 1,
            flags: vec![0, 0b1000],
            vars: vec![7, 0xBEEF],
            taken_ms: 3,
        };
        assert_eq!(snapshot.flag(11), Some(true));
        assert_eq!(snapshot.flag(10), Some(false));
        assert_eq!(snapshot.flag(16), None);
        assert_eq!(snapshot.var(0x4001), Some(0xBEEF));
        assert_eq!(snapshot.var(0x4002), None);
        assert_eq!(snapshot.var(0x3FFF), None);
        assert_eq!(snapshot.value(Target::Flag { number: 11 }), Some(1));
        assert_eq!(snapshot.value(Target::Var { number: 0x4000 }), Some(7));
        assert_eq!(
            serde_json::to_string(&FLAG_173).unwrap(),
            r#"{"kind":"flag","number":371}"#
        );
        assert_eq!(
            serde_json::from_str::<Target>(r#"{"kind":"var","number":16549}"#).unwrap(),
            VAR_A5
        );
        assert_eq!(FLAG_173.kind(), FlagKind::Flag);
        assert_eq!(VAR_A5.number(), 0x40A5);
    }

    #[test]
    fn connect_verifies_the_game_code_and_continues() {
        let stub = platinum(Quirks::melonds_1_1());
        let started = Instant::now();
        let session = session(&stub);
        let took = started.elapsed();
        println!("connect took {took:?}");
        if strict() {
            assert!(took <= Duration::from_millis(100), "connect took {took:?}");
        }
        assert_eq!(session.game_code(), "CPUD");
        assert!(session.has_save());
        assert!(session.last().is_none());
        assert!(session.halt().is_none());
        runs(&stub);
        let log = stub.log_lines();
        assert_eq!(
            log[3..],
            [
                format!("m{GAME_CODE_ADDR:08x},4"),
                format!("m{POINTER:08x},4"),
                "c".to_string()
            ],
            "{log:?}"
        );
    }

    #[test]
    fn connect_refuses_another_game() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(GAME_CODE_ADDR, b"IPKD");
        let result = LiveSession::connect(&stub.addr(), "CPUD", REPLY_TIMEOUT);
        match result {
            Err(LiveError::WrongGame { found, expected }) => {
                assert_eq!(found, "IPKD");
                assert_eq!(expected, "CPUD");
            }
            other => panic!("expected WrongGame, got {other:?}"),
        }
        let log = stub.log_lines();
        assert!(!log.iter().any(|l| l.starts_with('M')), "{log:?}");
        // The refused session does not leave the game halted.
        runs(&stub);
    }

    #[test]
    fn connect_without_ascii_code_is_unverified() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(GAME_CODE_ADDR, &[0; 4]);
        let (mut session, check) =
            LiveSession::connect(&stub.addr(), "CPUD", REPLY_TIMEOUT).unwrap();
        assert_eq!(check, GameCheck::Unverified);
        assert!(session.has_save());
        stub.poke(VAR_40A5, &[9, 0]);
        assert_eq!(session.refresh().unwrap().var(0x40A5), Some(9));
    }

    #[test]
    fn a_project_without_a_save_layout_is_refused_before_connecting() {
        let stub = platinum(Quirks::melonds_1_1());
        let result = LiveSession::connect(&stub.addr(), "ADAE", REPLY_TIMEOUT);
        assert!(
            matches!(&result, Err(LiveError::Io(e)) if e.kind() == std::io::ErrorKind::Unsupported),
            "{result:?}"
        );
        assert!(stub.log_lines().is_empty());
    }

    #[test]
    fn connect_before_the_save_is_loaded_keeps_waiting() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(POINTER, &[0; 4]);
        let mut session = session(&stub);
        assert!(!session.has_save());
        runs(&stub);
        // Without a save nothing is written, a watch can still be set up.
        assert!(matches!(
            session.set(FLAG_173, 1),
            Err(LiveError::NoSave)
        ));
        let watch = session.add_watch(VAR_A5, "Variable".into()).unwrap();
        assert_eq!(watch.id, 1);
        assert!(matches!(session.refresh(), Err(LiveError::NoSave)));
        assert!(!session.has_save());
        runs(&stub);
        assert!(!stub.log_lines().iter().any(|l| l.starts_with('M')));

        stub.poke(POINTER, &BLOCK.to_le_bytes());
        stub.poke(VAR_40A5, &[1, 0]);
        let snapshot = session.refresh().unwrap();
        assert_eq!(snapshot.var(0x40A5), Some(1));
        assert_eq!(snapshot.revision, 1);
        assert!(session.has_save());
        runs(&stub);
    }

    #[test]
    fn refresh_is_a_halt_read_continue_cycle_from_one_frame() {
        let stub = platinum(Quirks::melonds_1_1());
        let mut session = session(&stub);
        runs(&stub);
        let before = stub.log_lines().len();
        stub.poke(FLAG_173_BYTE, &[0b1000]);
        stub.poke(VAR_40A5, &[0x34, 0x12]);
        let started = Instant::now();
        let snapshot = session.refresh().unwrap().clone();
        println!("snapshot took {:?} ({} ms)", started.elapsed(), snapshot.taken_ms);
        assert_eq!(snapshot.flags.len(), 364);
        assert_eq!(snapshot.vars.len(), 288);
        assert_eq!(snapshot.flag(0x173), Some(true));
        assert_eq!(snapshot.flag(0x172), Some(false));
        assert_eq!(snapshot.var(0x40A5), Some(0x1234));
        assert_eq!(snapshot.revision, 1);
        assert_eq!(session.last(), Some(&snapshot));
        wait_until("the c", || stub.log_lines().len() == before + 6);
        let log = stub.log_lines()[before..].to_vec();
        assert_eq!(
            log,
            [
                "\x03".to_string(),
                format!("m{POINTER:08x},4"),
                format!("m{VARS:08x},{CHUNK:x}"),
                format!("m{:08x},40", VARS + CHUNK as u32),
                format!("m{FLAGS:08x},16c"),
                "c".to_string(),
            ],
        );
        runs(&stub);
        assert_eq!(session.refresh().unwrap().revision, 2);
    }

    #[test]
    fn set_flag_changes_one_bit_and_reads_back() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(FLAG_173_BYTE, &[0xFF]);
        let mut session = session(&stub);
        assert_eq!(session.read_back(), None);
        session.set(FLAG_173, 0).unwrap();
        assert_eq!(stub.peek(FLAG_173_BYTE, 1), vec![0xF7]);
        assert_eq!(session.read_back(), None);
        runs(&stub);
        assert_eq!(session.refresh().unwrap().flag(0x173), Some(false));
        assert_eq!(session.read_back(), Some(true));
        // The read-back stays until the next write.
        session.refresh().unwrap();
        assert_eq!(session.read_back(), Some(true));
        session.set(FLAG_173, 1).unwrap();
        assert_eq!(session.read_back(), None);
        assert_eq!(stub.peek(FLAG_173_BYTE, 1), vec![0xFF]);
        let writes: Vec<String> = stub
            .log_lines()
            .into_iter()
            .filter(|l| l.starts_with('M'))
            .collect();
        assert_eq!(
            writes,
            [
                format!("M{FLAG_173_BYTE:08x},1:f7"),
                format!("M{FLAG_173_BYTE:08x},1:ff")
            ]
        );
    }

    #[test]
    fn set_var_writes_the_word_little_endian() {
        let stub = platinum(Quirks::melonds_1_1());
        let mut session = session(&stub);
        session.set(VAR_A5, 0x1234).unwrap();
        assert_eq!(stub.peek(VAR_40A5, 2), vec![0x34, 0x12]);
        runs(&stub);
        assert_eq!(session.refresh().unwrap().var(0x40A5), Some(0x1234));
        assert_eq!(session.read_back(), Some(true));
    }

    #[test]
    fn a_write_the_game_undoes_is_reported_not_hidden() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(FLAG_173_BYTE, &[0xFF]);
        let mut session = session(&stub);
        session.set(FLAG_173, 0).unwrap();
        stub.poke(FLAG_173_BYTE, &[0xFF]);
        session.refresh().unwrap();
        assert_eq!(session.read_back(), Some(false));
        assert_eq!(session.last().unwrap().flag(0x173), Some(true));
    }

    #[test]
    fn set_outside_the_save_is_refused() {
        let stub = platinum(Quirks::melonds_1_1());
        let mut session = session(&stub);
        let result = session.set(Target::Var { number: 0x4000 + 300 }, 1);
        assert!(
            matches!(
                result,
                Err(LiveError::OutsideSave {
                    kind: FlagKind::Var,
                    number: 0x412C
                })
            ),
            "{result:?}"
        );
        let result = session.set(Target::Flag { number: 364 * 8 }, 1);
        assert!(
            matches!(
                result,
                Err(LiveError::OutsideSave {
                    kind: FlagKind::Flag,
                    ..
                })
            ),
            "{result:?}"
        );
        let result = session.add_watch(Target::Var { number: 0x8000 }, "lokal".into());
        assert!(
            matches!(result, Err(LiveError::OutsideSave { .. })),
            "{result:?}"
        );
        assert!(session.watches().is_empty());
        assert!(!stub.log_lines().iter().any(|l| l.starts_with('M')));
        runs(&stub);
    }

    #[test]
    fn a_watch_halts_when_the_value_changes_and_resume_continues() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(VAR_40A5, &[1, 0]);
        let mut session = session(&stub);
        let watch = session
            .add_watch(VAR_A5, "Auslöser 2 · Eventdatei 394".into())
            .unwrap();
        assert_eq!(session.watches(), std::slice::from_ref(&watch));
        assert_eq!(session.refresh().unwrap().var(0x40A5), Some(1));
        assert!(session.halt().is_none());
        runs(&stub);

        stub.poke(VAR_40A5, &[0, 0]);
        let resumes = count(&stub, "c");
        session.refresh().unwrap();
        assert_eq!(
            session.halt(),
            Some(&Halt {
                watch: watch.clone(),
                before: 1,
                after: 0,
                revision: 2
            })
        );
        assert!(stub.halted.load(Ordering::SeqCst));
        // A refresh while halted reads but does not continue.
        session.refresh().unwrap();
        assert!(session.halt().is_some());
        assert!(stub.halted.load(Ordering::SeqCst));
        assert_eq!(count(&stub, "c"), resumes, "no c while halted");

        session.resume().unwrap();
        assert!(session.halt().is_none());
        runs(&stub);
        assert!(session.remove_watch(watch.id));
        assert!(!session.remove_watch(watch.id));
        assert!(session.watches().is_empty());
    }

    #[test]
    fn our_own_write_to_a_watched_value_does_not_halt() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(VAR_40A5, &[1, 0]);
        let mut session = session(&stub);
        session.add_watch(VAR_A5, "Variable".into()).unwrap();
        session.add_watch(FLAG_173, "Flag".into()).unwrap();
        session.refresh().unwrap();
        session.set(VAR_A5, 5).unwrap();
        session.refresh().unwrap();
        assert_eq!(session.halt(), None);
        assert_eq!(session.read_back(), Some(true));
        runs(&stub);

        // The game overwrites our write with another value: that halts.
        session.set(VAR_A5, 6).unwrap();
        stub.poke(VAR_40A5, &[7, 0]);
        session.refresh().unwrap();
        assert_eq!(session.read_back(), Some(false));
        let halt = session.halt().unwrap();
        assert_eq!((halt.before, halt.after), (5, 7));
        assert_eq!(halt.watch.id, 1);
        assert!(stub.halted.load(Ordering::SeqCst));

        // A write while halted keeps the game halted.
        session.set(FLAG_173, 1).unwrap();
        assert!(stub.halted.load(Ordering::SeqCst));
        assert!(session.halt().is_some());
        session.resume().unwrap();
        runs(&stub);
    }

    #[test]
    fn a_reset_to_the_title_drops_the_view_and_the_game_runs_on() {
        let stub = platinum(Quirks::melonds_1_1());
        let mut session = session(&stub);
        session.refresh().unwrap();
        stub.poke(POINTER, &[0; 4]);
        assert!(matches!(session.refresh(), Err(LiveError::NoSave)));
        assert!(!session.has_save());
        assert!(session.last().is_none());
        runs(&stub);

        // The save loads again, the block moved (heap).
        stub.poke(POINTER, &(BLOCK + 0x40).to_le_bytes());
        stub.poke(VAR_40A5 + 0x40, &[3, 0]);
        let snapshot = session.refresh().unwrap();
        assert_eq!(snapshot.var(0x40A5), Some(3));
        assert_eq!(snapshot.revision, 2);
        runs(&stub);
    }

    #[test]
    fn a_silent_stub_turns_into_lost_within_two_seconds_and_blocks_nothing() {
        // The connect of a session takes three packets after the handshake (game code,
        // pointer, `c`); then the stub goes silent.
        let stub = platinum(Quirks {
            answer_after: Some(3),
            ..Quirks::melonds_1_1()
        });
        let mut session = session(&stub);
        let started = Instant::now();
        let result = session.refresh();
        let took = started.elapsed();
        println!("silent stub: {result:?} after {took:?}");
        assert!(matches!(result, Err(LiveError::NoReply)), "{result:?}");
        assert!(took < bound(1500), "{took:?}");
        // The caller drops the session: the stub sees the disconnect and the game runs.
        drop(session);
        runs(&stub);
        wait_until("the connection to end", || stub.open_connections() == 0);
    }

    #[test]
    fn a_stub_that_accepts_but_never_answers_is_unavailable_after_the_connect_deadline() {
        let deadline = Duration::from_millis(300);
        // A stub that never answers, and melonDS busy with another client.
        let silent = platinum(Quirks {
            answer: false,
            ..Quirks::melonds_1_1()
        });
        let busy = platinum(Quirks::melonds_1_1());
        let _first = session(&busy);
        for stub in [&silent, &busy] {
            let started = Instant::now();
            let result = LiveSession::connect(&stub.addr(), "CPUD", deadline);
            let took = started.elapsed();
            println!("no answer: {result:?} after {took:?}");
            assert!(matches!(result, Err(LiveError::NoReply)), "{result:?}");
            assert!(took >= deadline, "{took:?}");
            assert!(took < bound(1000), "{took:?}");
        }
    }

    #[test]
    fn detach_lets_the_game_run_even_when_halted() {
        let stub = platinum(Quirks::melonds_1_1());
        stub.poke(VAR_40A5, &[1, 0]);
        let mut session = session(&stub);
        session.add_watch(VAR_A5, "Variable".into()).unwrap();
        session.refresh().unwrap();
        stub.poke(VAR_40A5, &[2, 0]);
        session.refresh().unwrap();
        assert!(session.halt().is_some());
        session.detach().unwrap();
        assert_eq!(count(&stub, "D"), 1);
        runs(&stub);
    }
}
