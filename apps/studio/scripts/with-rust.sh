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
export PATH
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo nicht gefunden: rustup installieren (brew install rustup) oder ~/.cargo/bin anlegen" >&2
  exit 127
fi
exec "$@"
