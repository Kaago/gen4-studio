#!/bin/bash
# Baut <name>.jsx aus jsx-lib-app.js und <name>.body.js (im aktuellen Ordner) für den render-Aufruf.
#   bash build-screen.sh <name> [--dunkel]
# --dunkel stellt dunkel-map.js voran: v('token') liefert dann die Hex-Werte des dunklen Themas.
D=$(cd "$(dirname "$0")" && pwd)
{ echo '{(() => {'; [ "$2" = "--dunkel" ] && grep -v '^//' "$D/dunkel-map.js"; grep -v '^//' "$D/jsx-lib-app.js"; cat "$1.body.js"; echo '})()}'; } > "$1.jsx"
echo "$1.jsx ($(wc -c < "$1.jsx") Bytes)"
