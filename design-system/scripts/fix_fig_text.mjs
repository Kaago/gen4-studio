// Repariert verstümmelte Texte in einer OpenPencil-.fig.
//
// Hintergrund (OpenPencil 0.16.0): Beim Speichern schreibt OpenPencil für manche Textknoten
// Glyphen-Umrisse (derivedTextData.glyphs[].commandsBlob) mit falschen Blob-Indizes. Beim Laden
// zeichnet es diese gespeicherten Umrisse statt der Schrift, und der Text erscheint als
// Zeichensalat. Ohne commandsBlob setzt OpenPencil den Text beim Laden selbst mit der echten Schrift.
// Das Skript entfernt deshalb nur die Umriss-Verweise der Textglyphen; Positionen, Inhalte,
// Schriften und alle anderen Daten bleiben unverändert.
//
// Aufruf (OpenPencil vorher schließen oder die Datei danach neu öffnen, nicht danach speichern):
//   node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555   (aus der Repo-Wurzel)
// Voraussetzung: @open-pencil/mcp ist global installiert (npm i -g @open-pencil/mcp), oder
// OPENPENCIL_KIWI zeigt auf das Paket @open-pencil/kiwi.
import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

// Mit --rgb555 rastet das Skript zusätzlich jede Flächen-, Kontur-, Verlaufs- und Effektfarbe auf
// den nächsten Wert des DS-Farbraums ein (Kanalwerte (v << 3) | (v >> 2)), höchstens ±4 je Kanal.
const args = process.argv.slice(2)
const snap555 = args.includes('--rgb555')
const fileArg = args.find((a) => !a.startsWith('--'))
const file = resolve(fileArg ?? '')
if (!fileArg) {
  console.error('Aufruf: node fix_fig_text.mjs <datei.fig> [--rgb555]')
  process.exit(2)
}

function kiwiPath() {
  if (process.env.OPENPENCIL_KIWI) return process.env.OPENPENCIL_KIWI
  const root = execFileSync('npm', ['root', '-g'], { encoding: 'utf8' }).trim()
  return join(root, '@open-pencil/mcp/node_modules/@open-pencil/kiwi')
}

const K = kiwiPath()
const codec = await import(join(K, 'dist/fig/codec.js'))
const container = await import(join(K, 'dist/fig/container.js'))
await codec.initCodec()

const work = mkdtempSync(join(tmpdir(), 'figfix-'))
execFileSync('unzip', ['-q', '-o', file, '-d', work])
const canvasPath = join(work, 'canvas.fig')
const bin = new Uint8Array(readFileSync(canvasPath))
const version = new DataView(bin.buffer, bin.byteOffset, bin.byteLength).getUint32(8, true)
const chunks = container.parseFigKiwiChunks(bin)
if (!chunks || chunks.length !== 2) throw new Error('Unerwartetes Containerformat in canvas.fig')

const message = codec.decodeMessage(container.decompressFigKiwiData(chunks[1]))
let texts = 0
let glyphs = 0
for (const nc of message.nodeChanges) {
  if (nc.type !== 'TEXT') continue
  let touched = false
  for (const g of nc.derivedTextData?.glyphs ?? []) {
    if (g.commandsBlob !== undefined) {
      delete g.commandsBlob
      glyphs++
      touched = true
    }
  }
  if (touched) texts++
}

let snapped = 0
if (snap555) {
  const steps = Array.from({ length: 32 }, (_, v) => (v << 3) | (v >> 2))
  const near = (x) => {
    const v = Math.round(x * 255)
    return steps.reduce((a, b) => (Math.abs(b - v) < Math.abs(a - v) ? b : a)) / 255
  }
  const fix = (c) => {
    if (!c) return
    const next = { r: near(c.r), g: near(c.g), b: near(c.b) }
    if (next.r !== c.r || next.g !== c.g || next.b !== c.b) {
      Object.assign(c, next)
      snapped++
    }
  }
  for (const nc of message.nodeChanges) {
    for (const key of ['fillPaints', 'strokePaints']) {
      for (const p of nc[key] ?? []) {
        fix(p.color)
        for (const s of p.stops ?? []) fix(s.color)
      }
    }
    for (const e of nc.effects ?? []) fix(e.color)
  }
}

const rebuilt = container.buildFigKiwi(chunks[0], codec.encodeMessage(message), version)
writeFileSync(canvasPath, rebuilt)

copyFileSync(file, file + '.vor-textfix')
const out = join(work, 'out.fig')
execFileSync('zip', ['-q', '-0', '-X', out, 'canvas.fig', 'thumbnail.png', 'meta.json'], { cwd: work })
copyFileSync(out, file)
console.log(`${texts} Textknoten repariert (${glyphs} Glyphen-Umrisse entfernt).${snap555 ? ` ${snapped} Farben auf RGB555 gerastet.` : ''} Sicherung: ${file}.vor-textfix`)
