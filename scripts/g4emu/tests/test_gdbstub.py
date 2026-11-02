"""The GDB stub server against a plain-socket client; a pump thread plays the emulation thread."""

from __future__ import annotations

import socket
import threading
import time

import pytest

from g4emu.gdbstub import FEATURES, GdbStubServer

RAM_BASE = 0x02000000
RAM_SIZE = 4 * 1024 * 1024


def _frame(payload: str) -> bytes:
    return f"${payload}#{sum(payload.encode()) % 256:02x}".encode()


def _read_packet(sock: socket.socket, timeout: float = 2.0, prefix: bytes = b"") -> str:
    """The next `$…#xx` payload; acks before it are skipped. `prefix`: bytes already read."""
    sock.settimeout(timeout)
    buf = prefix
    while True:
        if b"$" in buf:
            start = buf.index(b"$")
            hash_at = buf.find(b"#", start)
            if hash_at != -1 and len(buf) >= hash_at + 3:
                payload = buf[start + 1 : hash_at]
                assert int(buf[hash_at + 1 : hash_at + 3], 16) == sum(payload) % 256
                return payload.decode()
        chunk = sock.recv(65536)
        if not chunk:
            raise ConnectionError("stub closed the connection")
        buf += chunk


def _rsp(sock: socket.socket, payload: str) -> str:
    """Sends one packet and returns the payload of its reply."""
    sock.sendall(_frame(payload))
    return _read_packet(sock)


@pytest.fixture
def ram() -> bytearray:
    return bytearray(RAM_SIZE)


@pytest.fixture
def server(ram):
    stub = GdbStubServer(lambda: memoryview(ram), port=0)
    stub.start()
    stop = threading.Event()

    def pump() -> None:  # the emulation thread: one service per "frame"
        while not stop.is_set():
            stub.service()
            time.sleep(0.001)

    thread = threading.Thread(target=pump, daemon=True)
    thread.start()
    yield stub
    stop.set()
    thread.join()
    stub.close()


def _connect(stub: GdbStubServer) -> socket.socket:
    sock = socket.create_connection(("127.0.0.1", stub.port), timeout=2)
    sock.sendall(b"+")
    return sock


def _until(what: str, cond, timeout: float = 2.0) -> None:
    deadline = time.monotonic() + timeout
    while not cond():
        assert time.monotonic() < deadline, f"timed out waiting for {what}"
        time.sleep(0.002)


def test_a_client_is_halted_until_c(server):
    assert server.port != 0
    assert not server.halted and not server.connected
    with _connect(server) as sock:
        _until("the client to be served", lambda: server.connected)
        assert server.halted
        assert _rsp(sock, "qSupported:swbreak+") == FEATURES
        assert server.halted
        sock.sendall(_frame("c"))
        _until("the game to run", lambda: not server.halted)
        assert server.connected


def test_reads_and_writes_go_to_the_ram(server, ram):
    ram[0x101EE0:0x101EE4] = bytes([0x54, 0xE3, 0x27, 0x02])
    with _connect(server) as sock:
        assert _rsp(sock, "m02101ee0,4") == "54e32702"
        assert _rsp(sock, "M0227e354,2:beef") == "OK"
        assert ram[0x27E354:0x27E356] == b"\xbe\xef"
        assert _rsp(sock, "m0227e354,2") == "beef"
        # Outside the 4 MiB main RAM.
        assert _rsp(sock, "m02400000,1") == "E01"
        assert _rsp(sock, "M01fffffe,2:0000") == "E01"
        assert _rsp(sock, "mzz") == "E01"


def test_ctrl_c_halts_and_answers_s05(server):
    # melonDS 1.1 answers `?` with S02 and the interrupt byte with S05 (probe-gdb-stub.md).
    with _connect(server) as sock:
        assert _rsp(sock, "?") == "S02"
        sock.sendall(_frame("c"))
        _until("the game to run", lambda: not server.halted)
        sock.sendall(b"\x03")
        assert _read_packet(sock) == "S05"
        assert server.halted


def test_qsupported_and_noack_mode(server):
    with _connect(server) as sock:
        sock.sendall(_frame("qSupported"))
        sock.settimeout(2)
        first = sock.recv(1)
        assert first == b"+"  # ack mode: the packet is acknowledged first
        assert _read_packet(sock) == FEATURES
        sock.sendall(b"+")
        assert _rsp(sock, "QStartNoAckMode") == "OK"
        sock.sendall(b"+")
        sock.sendall(_frame("m02000000,2"))
        sock.settimeout(2)
        assert sock.recv(1) == b"$"  # no ack before the reply any more
        assert _read_packet(sock, prefix=b"$") == "0000"
        assert _rsp(sock, "vCont?") == "vCont;c;s;t"
        assert _rsp(sock, "qUnknownThing") == ""


def test_detach_answers_ok_and_frees_the_slot(server):
    first = _connect(server)
    _until("the first client to be served", lambda: server.connected)
    assert _rsp(first, "D") == "OK"
    _until("the slot to be free", lambda: not server.connected)
    assert not server.halted  # like melonDS: the game runs after a detach
    first.close()
    with _connect(server) as second:
        assert _rsp(second, "qSupported") == FEATURES
        _until("the second client to be served", lambda: server.connected)
        assert server.halted


def test_a_client_that_goes_away_lets_the_game_run(server):
    sock = _connect(server)
    _until("the client to be served", lambda: server.connected and server.halted)
    sock.close()
    _until("the slot to be free", lambda: not server.connected)
    assert not server.halted


def test_a_second_client_waits_unserved(server):
    with _connect(server) as first:
        assert _rsp(first, "qSupported") == FEATURES
        with _connect(server) as second:
            second.sendall(_frame("qSupported"))
            second.settimeout(0.3)
            with pytest.raises(TimeoutError):
                second.recv(1)
            # The first client is still served.
            assert _rsp(first, "m02000000,1") == "00"
            first.sendall(_frame("D"))
            assert _read_packet(first) == "OK"
            # Now the queued one is served, its waiting packet included.
            assert _read_packet(second) == FEATURES


def test_reads_over_512_bytes_are_served_in_full(server, ram):
    ram[0:2048] = bytes(i % 251 for i in range(2048))
    with _connect(server) as sock:
        assert _rsp(sock, "m02000000,800") == bytes(ram[0:2048]).hex()
        data = bytes(range(256)) * 3
        assert _rsp(sock, f"M02001000,{len(data):x}:{data.hex()}") == "OK"
        assert bytes(ram[0x1000 : 0x1000 + len(data)]) == data


def test_a_bad_checksum_gets_a_nak(server):
    with _connect(server) as sock:
        sock.sendall(b"$m02000000,1#00")
        sock.settimeout(2)
        assert sock.recv(1) == b"-"


def test_service_counts_the_packets_and_close_ends_everything(ram):
    stub = GdbStubServer(lambda: memoryview(ram), port=0)
    stub.start()
    try:
        with _connect(stub) as sock:
            _until("the client to be accepted", lambda: stub.service() >= 0 and stub.connected)
            sock.sendall(_frame("?") + _frame("m02000000,1") + b"\x03")
            time.sleep(0.05)
            assert stub.service() == 3
            assert stub.service() == 0
    finally:
        stub.close()
    with pytest.raises(OSError):
        socket.create_connection(("127.0.0.1", stub.port), timeout=0.5).close()
