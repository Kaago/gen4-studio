#!/usr/bin/env bash
# Guard against ROM data in the repository: nothing that is, or is derived from, a game ROM or
# save game may reach GitHub (spec release-ci 7.1).
#
#   check-no-rom-data.sh                      the files of `git ls-files` (cwd = the repo)
#   check-no-rom-data.sh --history <range>    every blob of a commit range (git rev-list --objects)
#   check-no-rom-data.sh <path>...            files and folders, recursively; a .dmg is mounted
#                                             read-only for the scan
#
# A hit is a file with the extension .nds, .sav, .dsv, .srl, .xdelta or .narc, a folder named
# like *_DSPRE_contents or *_testkopien, or any file of 0x160 bytes or more with the NDS header
# checksum of the Nintendo logo (0xCF56, little endian) at 0x15C, whatever the file is called.
# There are no exceptions and no allow list: a false hit makes the rule more precise.
#
# Output: `ROM? <path>: <reason>` per hit and exit status 1; otherwise the last line is
# `no rom data: <n> files ok`. Needs python3 (standard library only); runs on macOS bash 3.2 and
# Git Bash on Windows.
set -euo pipefail

if ! command -v python3 >/dev/null 2>&1; then
  echo "check-no-rom-data: python3 fehlt" >&2
  exit 2
fi

# -I: isolated mode, so a stray module in the current directory is never imported.
exec python3 -I - "$@" <<'PY'
import os
import subprocess
import sys
import tempfile
import threading

EXTENSIONS = (".nds", ".sav", ".dsv", ".srl", ".xdelta", ".narc")
FOLDER_MARKERS = ("_DSPRE_contents", "_testkopien")
HEADER_END = 0x15E  # the two checksum bytes sit at 0x15C and 0x15D
HEADER_MIN_SIZE = 0x160
NDS_LOGO_CRC = b"\x56\xcf"

USAGE = "usage: check-no-rom-data.sh [--history <rev-range> | <path>...]"


def path_reasons(path):
    """Reasons for a repo-relative path (extension, folder name); '/' separated."""
    reasons = []
    parts = path.replace("\\", "/").split("/")
    for marker in FOLDER_MARKERS:
        # Only the folder names count, not the file name itself.
        if any(marker in part for part in parts[:-1]):
            reasons.append("folder " + marker)
    ext = os.path.splitext(parts[-1])[1].lower()
    if ext in EXTENSIONS:
        reasons.append("extension " + ext)
    return reasons


def header_matches(head, size):
    return size >= HEADER_MIN_SIZE and head[0x15C:0x15E] == NDS_LOGO_CRC


class Report:
    def __init__(self):
        self.checked = 0
        self.hits = 0

    def hit(self, label, reason):
        self.hits += 1
        print("ROM? %s: %s" % (label, reason))

    def check(self, label, reasons, head=None, size=0):
        """Records one file: reasons from the path plus the header check of `head`."""
        self.checked += 1
        reasons = list(reasons)
        if head is not None and header_matches(head, size):
            reasons.append("nds header")
        for reason in reasons:
            self.hit(label, reason)

    def finish(self):
        sys.stdout.flush()
        if self.hits:
            print("%d hit(s), no rom data: FAILED" % self.hits, file=sys.stderr)
            return 1
        print("no rom data: %d files ok" % self.checked)
        return 0


def read_head(path):
    """(first 0x15E bytes, size) of a regular file; None when it cannot be read."""
    try:
        with open(path, "rb") as f:
            size = os.fstat(f.fileno()).st_size
            return f.read(HEADER_END), size
    except OSError as err:
        print("check-no-rom-data: %s: %s" % (path, err.strerror or err), file=sys.stderr)
        return None


def scan_files(report, items):
    """items: (label, rel, real path); the path rules look at `rel`, the output shows `label`."""
    for label, rel, real in items:
        if os.path.islink(real) or not os.path.isfile(real):
            continue
        res = read_head(real)
        if res is None:
            report.hit(label, "unreadable")
            continue
        report.check(label, path_reasons(rel), res[0], res[1])


def walk(base_label, base):
    """Yields (label, rel, real path) of all files below a directory (symlinks not followed)."""
    for dirpath, dirnames, filenames in os.walk(base):
        dirnames.sort()
        rel = os.path.relpath(dirpath, base)
        for name in sorted(filenames):
            real = os.path.join(dirpath, name)
            sub = name if rel == "." else rel.replace(os.sep, "/") + "/" + name
            yield base_label + "/" + sub, sub, real


def scan_dmg(report, dmg):
    mount = tempfile.mkdtemp(prefix="g4-dmg-")
    attached = False
    try:
        subprocess.run(
            ["hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", mount, dmg],
            check=True, stdout=subprocess.DEVNULL,
        )
        attached = True
        # The mount point is a temp folder, so the content is labelled <dmg>/<path in the image>.
        scan_files(report, walk(dmg, mount))
    finally:
        if attached:
            if subprocess.run(["hdiutil", "detach", mount], stdout=subprocess.DEVNULL,
                              stderr=subprocess.DEVNULL).returncode != 0:
                subprocess.run(["hdiutil", "detach", "-force", mount],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            os.rmdir(mount)
        except OSError:
            pass


def scan_paths(report, paths):
    for arg in paths:
        if not os.path.lexists(arg):
            print("check-no-rom-data: %s: not found" % arg, file=sys.stderr)
            sys.exit(2)
        if os.path.isdir(arg):
            base = arg.rstrip("/\\") or arg
            # The folder name of the argument itself counts too (reported once).
            own = os.path.basename(os.path.abspath(base))
            for marker in FOLDER_MARKERS:
                if marker in own:
                    report.hit(base, "folder " + marker)
            scan_files(report, walk(base, base))
        elif arg.lower().endswith(".dmg"):
            scan_dmg(report, arg)
        else:
            scan_files(report, [(arg, os.path.basename(arg), arg)])


def git_lines(args):
    return subprocess.run(["git"] + args, check=True, stdout=subprocess.PIPE).stdout


def scan_repo_files(report):
    try:
        top = git_lines(["rev-parse", "--show-toplevel"]).decode().strip()
    except (subprocess.CalledProcessError, OSError):
        print("check-no-rom-data: not inside a git repository", file=sys.stderr)
        sys.exit(2)
    names = git_lines(["-C", top, "ls-files", "-z"]).split(b"\0")
    items = []
    for raw in names:
        if raw:
            label = os.fsdecode(raw)
            items.append((label, label, os.path.join(top, label)))
    scan_files(report, items)


def scan_history(report, rev_range):
    try:
        listing = git_lines(["rev-list", "--objects", rev_range])
    except (subprocess.CalledProcessError, OSError):
        print("check-no-rom-data: git rev-list failed for %r" % rev_range, file=sys.stderr)
        sys.exit(2)
    objects = []  # (sha, path); commits and the root tree have no path
    for line in listing.decode("utf-8", "surrogateescape").splitlines():
        sha, _, path = line.partition(" ")
        if path:
            objects.append((sha, path))

    # One process for type and size of everything, one for the contents of the blobs that can
    # carry a header. Trees show up here too (with their folder name) and are skipped.
    proc = subprocess.run(
        ["git", "cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize)"],
        input="".join(sha + "\n" for sha, _ in objects).encode(),
        check=True, stdout=subprocess.PIPE,
    )
    info = {}
    for line in proc.stdout.decode().splitlines():
        sha, kind, size = line.split()
        info[sha] = (kind, int(size))

    to_read = []
    for sha, path in objects:
        kind, size = info.get(sha, ("", 0))
        if kind != "blob":
            continue
        reasons = path_reasons(path)
        if size < HEADER_MIN_SIZE:
            report.check("%s (%s)" % (path, sha[:7]), reasons)
        else:
            to_read.append((sha, path, reasons, size))

    if not to_read:
        return
    cat = subprocess.Popen(["git", "cat-file", "--batch"], stdin=subprocess.PIPE,
                           stdout=subprocess.PIPE)

    def feed():
        for sha, _, _, _ in to_read:
            cat.stdin.write((sha + "\n").encode())
        cat.stdin.close()

    writer = threading.Thread(target=feed)
    writer.start()
    for sha, path, reasons, size in to_read:
        header = cat.stdout.readline().split()
        if len(header) != 3 or header[0].decode() != sha:
            print("check-no-rom-data: unexpected git cat-file output for %s" % sha, file=sys.stderr)
            sys.exit(2)
        remaining = int(header[2])
        head = cat.stdout.read(min(remaining, HEADER_END))
        remaining -= len(head)
        while remaining > 0:  # drain the rest of the blob without keeping it
            chunk = cat.stdout.read(min(remaining, 1 << 20))
            if not chunk:
                break
            remaining -= len(chunk)
        cat.stdout.read(1)  # the newline after each object
        report.check("%s (%s)" % (path, sha[:7]), reasons, head, size)
    writer.join()
    cat.wait()


def main(argv):
    report = Report()
    if argv and argv[0] == "--history":
        if len(argv) != 2:
            print(USAGE, file=sys.stderr)
            return 2
        scan_history(report, argv[1])
    elif argv and argv[0].startswith("-") and argv[0] != "-":
        print(USAGE, file=sys.stderr)
        return 2
    elif argv:
        scan_paths(report, argv)
    else:
        scan_repo_files(report)
    return report.finish()


sys.exit(main(sys.argv[1:]))
PY
