#!/usr/bin/env python3
"""Probe des GDB-Stubs von melonDS 1.1 fuer Teilphase 3d.9 ("Spielmodus live", Weg 3).

Nur Standardbibliothek. Spricht das GDB-Remote-Protokoll ($payload#xx, +-Acks,
qSupported, ?, m, M, c, \\x03, D, QStartNoAckMode) mit dem ARM9-Stub (Port 3333).

Aufrufe (melonDS muss mit aktiviertem Stub laufen, Instance0.Gdb.Enabled = true):
  probe-gdb-stub.py hello                      Verbindung, qSupported, ?, Zeiger lesen
  probe-gdb-stub.py findcounter                Bildzaehler im RAM suchen (u32, +~60/s)
  probe-gdb-stub.py measure --game pt --counter 0x...   alle Messungen, JSON auf stdout
  probe-gdb-stub.py reconnect --counter 0x...  nur neu verbinden und Zaehler lesen
  probe-gdb-stub.py poke --game pt --flag N --value 0|1  ein Flag setzen (sichtbarer Effekt)

Befunde stehen in probe-gdb-stub.md im selben Ordner.
"""
import argparse
import json
import socket
import statistics
import struct
import sys
import time

# Spielstand-Lage aus Gutachten B (gemessen): Zeiger, Kopf, Variablen, Flags.
GAMES = {
    "pt": dict(pointer=0x02101EE0, header=0x14, vars=0xDAC, flags=0xFEC),     # Platin DE (CPUD)
    "hg": dict(pointer=0x02111860, header=0x10, vars=0xDE4, flags=0x10C4),    # HeartGold DE (IPKD)
    "hgus": dict(pointer=0x02111880, header=0x10, vars=0xDE4, flags=0x10C4),  # HeartGold US (IPKE)
}
NUM_VARS = 288
FLAG_BYTES = 0xB60 // 8  # 364 Byte
# Stub-Puffer 1152 Zeichen: m bis 575 Byte (576 -> keine Antwort, ab 577 E02);
# ein M-Paket ueber 1151 Zeichen legt den Stub bis zum Neustart von melonDS lahm.
MAX_CHUNK = 512


class GdbError(Exception):
    pass


class Gdb:
    def __init__(self, host="127.0.0.1", port=3333, timeout=3.0):
        t0 = time.perf_counter()
        self.sock = socket.create_connection((host, port), timeout)
        self.sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.sock.settimeout(timeout)
        self.buf = b""
        self.noack = False
        # Der Stub erwartet innerhalb von 1 s nach accept() ein '+', sonst trennt er.
        self.sock.sendall(b"+")
        self.connect_s = time.perf_counter() - t0

    def close(self):
        try:
            self.sock.close()
        except OSError:
            pass

    def _fill(self, timeout):
        self.sock.settimeout(timeout)
        data = self.sock.recv(65536)
        if not data:
            raise EOFError("Stub hat die Verbindung geschlossen")
        self.buf += data

    def read_packet(self, timeout=3.0):
        deadline = time.perf_counter() + timeout
        while True:
            # Acks/Naks vor dem Paket ueberspringen
            i = 0
            while i < len(self.buf) and self.buf[i:i + 1] in (b"+", b"-"):
                i += 1
            self.buf = self.buf[i:]
            if self.buf.startswith(b"$"):
                h = self.buf.find(b"#")
                if h >= 0 and len(self.buf) >= h + 3:
                    payload = self.buf[1:h]
                    ck = int(self.buf[h + 1:h + 3], 16)
                    self.buf = self.buf[h + 3:]
                    if sum(payload) & 0xFF != ck:
                        raise GdbError("Pruefsumme falsch")
                    if not self.noack:
                        self.sock.sendall(b"+")  # Stub blockiert bis zu 2 s auf dieses Ack
                    return payload.decode("latin-1")
            elif self.buf:
                raise GdbError("unerwartete Bytes: %r" % self.buf[:20])
            left = deadline - time.perf_counter()
            if left <= 0:
                raise socket.timeout("keine Antwort")
            self._fill(left)

    def send(self, payload):
        p = payload.encode("latin-1")
        self.sock.sendall(b"$" + p + b"#%02x" % (sum(p) & 0xFF))

    def cmd(self, payload, timeout=3.0):
        self.send(payload)
        return self.read_packet(timeout)

    def read_mem(self, addr, n, timeout=3.0):
        out = b""
        while n > 0:
            k = min(n, MAX_CHUNK)
            r = self.cmd("m%x,%x" % (addr, k), timeout)
            if r.startswith("E") and len(r) == 3:
                raise GdbError("m %x,%x -> %s" % (addr, k, r))
            out += bytes.fromhex(r)
            addr += k
            n -= k
        return out

    def write_mem(self, addr, data, timeout=3.0):
        while data:
            k = data[:MAX_CHUNK]
            r = self.cmd("M%x,%x:%s" % (addr, len(k), k.hex()), timeout)
            if r != "OK":
                raise GdbError("M %x -> %s" % (addr, r))
            addr += len(k)
            data = data[len(k):]

    def u32(self, addr):
        return struct.unpack("<I", self.read_mem(addr, 4))[0]

    def u16(self, addr):
        return struct.unpack("<H", self.read_mem(addr, 2))[0]

    def interrupt(self, timeout=3.0):
        self.sock.sendall(b"\x03")
        return self.read_packet(timeout)  # Stopp-Antwort, z. B. S02

    def cont(self):
        self.send("c")  # keine Antwort bis zum naechsten Stopp

    def detach(self):
        try:
            return self.cmd("D", 2.0)
        except (socket.timeout, EOFError) as e:
            return "keine Antwort (%s)" % e.__class__.__name__

    def start_noack(self):
        r = self.cmd("QStartNoAckMode")
        if r == "OK":
            self.noack = True
        return r


def save_block(g, game):
    lay = GAMES[game]
    ptr = g.u32(lay["pointer"])
    return ptr, ptr + lay["header"]


def is_running(g, counter, wait=0.3):
    a = g.u32(counter)
    time.sleep(wait)
    b = g.u32(counter)
    return b != a, a, b


def cmd_hello(a):
    g = Gdb(port=a.port)
    print("connect %.1f ms" % (g.connect_s * 1000))
    for p in ("qSupported:swbreak+;hwbreak+;vContSupported+", "vCont?", "?", "qAttached"):
        t = time.perf_counter()
        r = g.cmd(p)
        print("%-45s -> %-70s %.2f ms" % (p, r, (time.perf_counter() - t) * 1000))
    ptr, base = save_block(g, a.game)
    print("Zeiger 0x%08x  Basis 0x%08x" % (ptr, base))
    g.cont()
    time.sleep(0.2)
    print("detach:", g.detach())
    g.close()


def cmd_findcounter(a):
    g = Gdb(port=a.port)
    g.cmd("qSupported:swbreak+")
    g.start_noack()
    lo, hi = 0x02000000, 0x02400000
    t = time.perf_counter()
    d0 = g.read_mem(lo, hi - lo, timeout=10)  # direkt nach dem Verbinden steht die Emulation
    dump_s = time.perf_counter() - t
    g.cont()
    w0 = time.perf_counter()
    time.sleep(1.0)
    g.interrupt()
    w1 = time.perf_counter()
    d1 = g.read_mem(lo, hi - lo, timeout=10)
    g.cont()
    secs = w1 - w0
    cands = []
    for off in range(0, hi - lo, 4):
        x = struct.unpack_from("<I", d0, off)[0]
        y = struct.unpack_from("<I", d1, off)[0]
        dlt = (y - x) & 0xFFFFFFFF
        if 0.8 * 60 * secs <= dlt <= 1.2 * 60 * secs + 2:
            cands.append((lo + off, x, y, dlt))
    print("4-MiB-Dump im Halt: %.0f ms; Wartezeit %.2f s" % (dump_s * 1000, secs))
    for c in cands[:40]:
        print("0x%08x  %10d -> %10d  (+%d)" % c)
    print("%d Kandidaten" % len(cands))
    g.detach()
    g.close()


def cmd_measure(a):
    res = {}
    lay = GAMES[a.game]
    cnt = a.counter
    g = Gdb(port=a.port)
    res["connect_ms"] = round(g.connect_s * 1000, 2)

    # 1. Haelt das Verbinden die Emulation an?
    run, x, y = is_running(g, cnt, 0.5)
    res["running_after_connect"] = run
    res["counter_after_connect"] = [x, y]
    res["qSupported"] = g.cmd("qSupported:swbreak+;hwbreak+;vContSupported+")
    res["question"] = g.cmd("?")

    # 2. Weiterlaufen lassen, dann Lesen bei laufender Emulation
    g.cont()
    time.sleep(0.5)
    lat, vals = [], []
    for _ in range(20):
        t = time.perf_counter()
        vals.append(g.u32(cnt))
        lat.append((time.perf_counter() - t) * 1000)
        time.sleep(0.05)
    res["read_running_ms_median"] = round(statistics.median(lat), 2)
    res["read_running_ms_max"] = round(max(lat), 2)
    res["read_running_counter_advances"] = vals[-1] - vals[0]
    res["read_running_monotonic"] = all(b > a_ for a_, b in zip(vals, vals[1:]))

    # Ganze Bloecke lesen bei laufender Emulation (Zeiger, 288 Variablen, 364 Byte Flags)
    t = time.perf_counter()
    ptr, base = save_block(g, a.game)
    varsb = g.read_mem(base + lay["vars"], NUM_VARS * 2)
    flagsb = g.read_mem(base + lay["flags"], FLAG_BYTES)
    res["read_block_running_ms"] = round((time.perf_counter() - t) * 1000, 2)
    res["pointer"] = "0x%08x" % ptr
    res["vars_nonzero"] = sum(1 for i in range(NUM_VARS) if varsb[2 * i:2 * i + 2] != b"\0\0")
    res["flags_set"] = sum(bin(b).count("1") for b in flagsb)

    # Bildrate mit verbundenem Client (Zaehler/Sekunde)
    c0, t0 = g.u32(cnt), time.perf_counter()
    time.sleep(2.0)
    c1, t1 = g.u32(cnt), time.perf_counter()
    res["fps_connected_running"] = round((c1 - c0) / (t1 - t0), 1)

    # 3. Halt-Lesen-Weiter, 20 Zyklen
    cyc, halt_wait, advance, stopr = [], [], [], set()
    last = None
    for _ in range(20):
        t = time.perf_counter()
        stopr.add(g.interrupt())
        th = time.perf_counter()
        c = g.u32(cnt)
        g.read_mem(base + lay["vars"], NUM_VARS * 2)
        g.read_mem(base + lay["flags"], FLAG_BYTES)
        g.cont()
        te = time.perf_counter()
        cyc.append((te - t) * 1000)
        halt_wait.append((th - t) * 1000)
        if last is not None:
            advance.append(c - last)
        last = c
        time.sleep(0.1)
    res["hrc_cycle_ms_median"] = round(statistics.median(cyc), 2)
    res["hrc_cycle_ms_max"] = round(max(cyc), 2)
    res["hrc_interrupt_to_stop_ms_median"] = round(statistics.median(halt_wait), 2)
    res["hrc_stop_replies"] = sorted(stopr)
    res["hrc_counter_advance_between_cycles"] = [min(advance), max(advance)]

    # 3b. dasselbe im No-Ack-Modus (nur im Halt erlaubt)
    g.interrupt()
    res["noack"] = g.start_noack()
    g.cont()
    time.sleep(0.2)
    cyc2 = []
    for _ in range(20):
        t = time.perf_counter()
        g.interrupt()
        g.read_mem(base + lay["vars"], NUM_VARS * 2)
        g.read_mem(base + lay["flags"], FLAG_BYTES)
        g.cont()
        cyc2.append((time.perf_counter() - t) * 1000)
        time.sleep(0.1)
    res["hrc_noack_cycle_ms_median"] = round(statistics.median(cyc2), 2)
    lat2 = []
    for _ in range(20):
        t = time.perf_counter()
        g.u32(cnt)
        lat2.append((time.perf_counter() - t) * 1000)
        time.sleep(0.05)
    res["read_running_noack_ms_median"] = round(statistics.median(lat2), 2)

    # 4. Schreiben bei laufender Emulation: letzte Variable (Index 287) und letztes Flag-Byte
    va = base + lay["vars"] + (NUM_VARS - 1) * 2
    old = g.u16(va)
    t = time.perf_counter()
    g.write_mem(va, struct.pack("<H", 0xBEEF))
    back = g.u16(va)
    res["write_var_roundtrip_ms"] = round((time.perf_counter() - t) * 1000, 2)
    time.sleep(1.0)
    res["write_var"] = dict(addr="0x%08x" % va, old=old, readback=back, after_1s=g.u16(va))
    g.write_mem(va, struct.pack("<H", old))
    res["write_var_restored"] = g.u16(va) == old
    fa = base + lay["flags"] + FLAG_BYTES - 1
    oldf = g.read_mem(fa, 1)[0]
    g.write_mem(fa, bytes([oldf ^ 0x80]))
    time.sleep(1.0)
    res["write_flag"] = dict(addr="0x%08x" % fa, old=oldf, after_1s=g.read_mem(fa, 1)[0])
    g.write_mem(fa, bytes([oldf]))
    res["write_flag_restored"] = g.read_mem(fa, 1)[0] == oldf

    # 5. Trennen
    res["detach_reply"] = g.detach()
    g.close()
    print(json.dumps(res, indent=1))


def cmd_reconnect(a):
    res = {}
    t = time.perf_counter()
    try:
        g = Gdb(port=a.port, timeout=a.timeout)
        res["tcp_connect_ms"] = round(g.connect_s * 1000, 2)
        try:
            res["qSupported"] = g.cmd("qSupported:swbreak+", a.timeout)
            run, x, y = is_running(g, a.counter, 0.5)
            res["running_after_connect"] = run
            res["counter"] = [x, y]
            if a.cont:
                g.cont()
                time.sleep(0.5)
                res["running_after_c"] = is_running(g, a.counter, 0.5)[0]
            if a.detach:
                res["detach_reply"] = g.detach()
        except (socket.timeout, EOFError, GdbError) as e:
            res["error"] = "%s: %s" % (e.__class__.__name__, e)
        g.close()
    except OSError as e:
        res["error"] = "%s: %s" % (e.__class__.__name__, e)
    res["elapsed_ms"] = round((time.perf_counter() - t) * 1000, 1)
    print(json.dumps(res))


def cmd_poke(a):
    lay = GAMES[a.game]
    g = Gdb(port=a.port)
    g.cmd("qSupported:swbreak+")
    g.cont()
    time.sleep(0.2)
    ptr, base = save_block(g, a.game)
    if a.flag is not None:
        addr = base + lay["flags"] + a.flag // 8
        b = g.read_mem(addr, 1)[0]
        nb = (b | (1 << (a.flag % 8))) if a.value else (b & ~(1 << (a.flag % 8)))
        g.write_mem(addr, bytes([nb]))
        print("Flag %d @0x%08x: %02x -> %02x (gelesen %02x)" % (a.flag, addr, b, nb, g.read_mem(addr, 1)[0]))
    if a.var is not None:
        addr = base + lay["vars"] + (a.var - 0x4000) * 2
        old = g.u16(addr)
        g.write_mem(addr, struct.pack("<H", a.value))
        print("Var 0x%x @0x%08x: %d -> %d (gelesen %d)" % (a.var, addr, old, a.value, g.u16(addr)))
    g.detach()
    g.close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=3333)
    ap.add_argument("--game", default="pt", choices=sorted(GAMES))
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("hello")
    sub.add_parser("findcounter")
    m = sub.add_parser("measure")
    m.add_argument("--counter", type=lambda s: int(s, 0), required=True)
    r = sub.add_parser("reconnect")
    r.add_argument("--counter", type=lambda s: int(s, 0), required=True)
    r.add_argument("--timeout", type=float, default=3.0)
    r.add_argument("--cont", action="store_true")
    r.add_argument("--detach", action="store_true")
    p = sub.add_parser("poke")
    p.add_argument("--flag", type=int)
    p.add_argument("--var", type=lambda s: int(s, 0))
    p.add_argument("--value", type=int, required=True)
    a = ap.parse_args()
    {"hello": cmd_hello, "findcounter": cmd_findcounter, "measure": cmd_measure,
     "reconnect": cmd_reconnect, "poke": cmd_poke}[a.cmd](a)


if __name__ == "__main__":
    main()
