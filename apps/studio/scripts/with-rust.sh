#!/usr/bin/env bash
# Startet einen Befehl mit der Rust-Toolchain im PATH. Auf diesem Mac kommt `cargo` aus dem
# keg-only rustup von Homebrew (/opt/homebrew/opt/rustup/bin), das eine normale Shell nicht im
# PATH hat; `tauri build` und `tauri dev` rufen `cargo metadata` und brechen sonst mit
# "failed to run 'cargo metadata' ... No such file or directory" ab. Genutzt von `pnpm bundle`
# und `pnpm tauri`.
set -euo pipefail
for dir in "/opt/homebrew/opt/rustup/bin" "$HOME/.cargo/bin"; do
  if [ -d "$dir" ]; then
    case ":$PATH:" in
      *":$dir:"*) ;;
      *) PATH="$dir:$PATH" ;;
    esac
  fi
done
# Windows (Git Bash): uxie builds nitroarc with MinGW gcc and make from MSYS2. On the runner
# msys2/setup-msys2 already puts them on PATH; locally this finds a default MSYS2 install. Appended,
# so Git's own bash and tools stay first (MSYS2's bash must not take over). MSYS2_LOCATION may
# name another install (Windows or POSIX path, as setup-msys2 prints it).
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    msys="${MSYS2_LOCATION:-C:\\msys64}"
    msys="$(cygpath -u "$msys" 2>/dev/null || printf '%s' "$msys")"
    for dir in "$msys/mingw64/bin" "$msys/usr/bin"; do
      if [ -d "$dir" ]; then
        case ":$PATH:" in
          *":$dir:"*) ;;
          *) PATH="$PATH:$dir" ;;
        esac
      fi
    done
    ;;
esac
export PATH
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo nicht gefunden: rustup installieren (brew install rustup) oder ~/.cargo/bin anlegen" >&2
  exit 127
fi
exec "$@"
