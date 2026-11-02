"""A GDB remote-serial-protocol stub over the emulator's main RAM, in the manner of melonDS 1.1.

`g4k live` and the studio talk to melonDS's ARM9 GDB stub; `g4emu repl` serves this stub so the
same chain runs headless (docs/research/2026-10-30-phase-3d/probe-gdb-stub.md).
"""

from __future__ import annotations

import select
import socket
import threading
from typing import Callable

RAM_BASE = 0x02000000
RAM_SIZE = 4 * 1024 * 1024
# The `qSupported` reply of melonDS 1.1.
FEATURES = "PacketSize=47F;qXfer:features:read+;swbreak-;hwbreak+;QStartNoAckMode+"
_ACCEPT_POLL = 0.02  # seconds between the accept thread's looks at the listener
_SEND_TIMEOUT = 1.0


def _checksum(payload: bytes) -> int:
    return sum(payload) % 256


class GdbStubServer:
    """The stub of melonDS 1.1 as far as `g4k live` uses it, over `ram()` (4 MiB at 0x02000000).

    Like melonDS: a served client halts the emulation from the accept until `c`/`vCont;c`; `\\x03`
    halts and answers `S05`, `?` answers `S02`; `D` answers `OK`, and after a detach or a
    disconnect the emulation runs on. One client at a time: a second one is not accepted (its TCP
    connect succeeds through the listen backlog) and stays unanswered until the first leaves.
    Packets are acknowledged with `+` until `QStartNoAckMode`. `m`/`M` outside main RAM get `E01`.

    Not the 1.1 original: reads and writes of any length are served in full (melonDS answers `m`
    up to 575 bytes and wedges on `M` packets above about 1150 characters); no client is dropped
    for a missing first `+`; the no-ack mode ends with the connection; nothing is logged.

    Threads: `start()` runs an accept thread that only hands a new connection over. Everything
    else happens in `service()`, which the emulation thread calls between frames (so RAM is only
    touched there); a client accepted meanwhile is served (`connected`, `halted`) from the next
    `service()` on, as melonDS looks for a client at the start of each frame.
    """

    def __init__(self, ram: Callable[[], memoryview], port: int = 0, host: str = "127.0.0.1") -> None:
        self._ram = ram
        self._host = host
        self.port = port
        self.halted = False
        self.connected = False
        self._listener: socket.socket | None = None
        self._client: socket.socket | None = None  # adopted by service()
        self._pending: socket.socket | None = None  # accepted, not yet adopted
        self._lock = threading.Lock()
        self._closed = threading.Event()
        self._thread: threading.Thread | None = None
        self._buf = b""
        self._no_ack = False

    def start(self) -> None:
        listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind((self._host, self.port))
        listener.listen(4)
        listener.settimeout(_ACCEPT_POLL)
        self._listener = listener
        self.port = listener.getsockname()[1]
        self._thread = threading.Thread(target=self._accept_loop, name="gdbstub-accept", daemon=True)
        self._thread.start()

    def _accept_loop(self) -> None:
        assert self._listener is not None
        while not self._closed.is_set():
            with self._lock:
                busy = self._client is not None or self._pending is not None
            if busy:
                self._closed.wait(_ACCEPT_POLL)
                continue
            try:
                conn, _ = self._listener.accept()
            except (TimeoutError, socket.timeout):
                continue
            except OSError:
                return  # the listener was closed
            conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            conn.settimeout(_SEND_TIMEOUT)
            with self._lock:
                if self._closed.is_set():
                    conn.close()
                    return
                self._pending = conn

    def service(self) -> int:
        """Handles every packet that has arrived (an interrupt byte counts as one); returns how
        many. Call it from the emulation thread."""
        with self._lock:
            if self._client is None and self._pending is not None:
                self._client, self._pending = self._pending, None
                self._buf = b""
                self._no_ack = False
                self.connected = True
                self.halted = True  # melonDS halts on connect until `c`
        if self._client is None:
            return 0
        if not self._receive():
            self._drop()
            return 0
        handled = 0
        while True:
            outcome = self._next_packet()
            if outcome is None:
                return handled
            handled += 1
            if outcome is False:
                self._drop()
                return handled

    def close(self) -> None:
        self._closed.set()
        if self._listener is not None:
            self._listener.close()
        if self._thread is not None:
            self._thread.join()
        with self._lock:
            for sock in (self._client, self._pending):
                if sock is not None:
                    sock.close()
            self._client = self._pending = None
        self.connected = False
        self.halted = False

    # --- connection ---------------------------------------------------------------------------

    def _receive(self) -> bool:
        """Reads what is waiting into the buffer; False once the client is gone."""
        sock = self._client
        assert sock is not None
        try:
            while select.select([sock], [], [], 0)[0]:
                chunk = sock.recv(65536)
                if not chunk:
                    return False
                self._buf += chunk
        except OSError:
            return False
        return True

    def _drop(self) -> None:
        """The client left or detached: like melonDS the emulation runs on, the slot is free."""
        with self._lock:
            if self._client is not None:
                self._client.close()
            self._client = None
        self._buf = b""
        self._no_ack = False
        self.connected = False
        self.halted = False

    def _send(self, data: bytes) -> bool:
        assert self._client is not None
        try:
            self._client.sendall(data)
        except OSError:
            return False
        return True

    def _reply(self, payload: str) -> bool:
        raw = payload.encode("ascii")
        return self._send(b"$" + raw + b"#" + f"{_checksum(raw):02x}".encode("ascii"))

    def _next_packet(self) -> bool | None:
        """Handles one item from the buffer: None when nothing complete is left, False when the
        connection ends (`D`, a failed send), else True."""
        self._buf = self._buf.lstrip(b"+-")
        if not self._buf:
            return None
        if self._buf[0] == 0x03:
            self._buf = self._buf[1:]
            self.halted = True
            return self._reply("S05")
        start = self._buf.find(b"$")
        if start == -1:
            self._buf = b""  # noise without a packet
            return None
        hash_at = self._buf.find(b"#", start)
        if hash_at == -1 or len(self._buf) < hash_at + 3:
            self._buf = self._buf[start:]
            return None
        payload = self._buf[start + 1 : hash_at]
        sum_text = self._buf[hash_at + 1 : hash_at + 3]
        self._buf = self._buf[hash_at + 3 :]
        try:
            ok = int(sum_text, 16) == _checksum(payload)
        except ValueError:
            ok = False
        if not ok:
            return self._no_ack or self._send(b"-")
        if not self._no_ack and not self._send(b"+"):
            return False
        return self._command(payload.decode("ascii", errors="replace"))

    # --- commands -----------------------------------------------------------------------------

    def _command(self, packet: str) -> bool:
        if packet.startswith("qSupported"):
            return self._reply(FEATURES)
        if packet == "QStartNoAckMode":
            sent = self._reply("OK")
            self._no_ack = True
            return sent
        if packet == "?":
            return self._reply("S02")
        if packet == "vCont?":
            return self._reply("vCont;c;s;t")
        if packet in ("c", "vCont;c"):
            self.halted = False
            return True
        if packet == "D":
            self._reply("OK")
            return False
        if packet.startswith("m"):
            return self._reply(self._read(packet[1:]))
        if packet.startswith("M"):
            return self._reply(self._write(packet[1:]))
        return self._reply("")

    def _range(self, text: str) -> tuple[int, int] | None:
        """Offset into RAM and length of `addr,len` (hex), if it lies inside main RAM."""
        addr_text, sep, len_text = text.partition(",")
        try:
            addr, length = int(addr_text, 16), int(len_text, 16)
        except ValueError:
            return None
        start = addr - RAM_BASE
        if not sep or start < 0 or start + length > RAM_SIZE:
            return None
        return start, length

    def _read(self, text: str) -> str:
        found = self._range(text)
        if found is None:
            return "E01"
        start, length = found
        return bytes(self._ram()[start : start + length]).hex()

    def _write(self, text: str) -> str:
        head, sep, data_hex = text.partition(":")
        found = self._range(head)
        if not sep or found is None:
            return "E01"
        start, length = found
        try:
            data = bytes.fromhex(data_hex)
        except ValueError:
            return "E01"
        if len(data) != length:
            return "E01"
        self._ram()[start : start + length] = data
        return "OK"
