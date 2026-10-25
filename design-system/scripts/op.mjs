// Ruft ein Werkzeug des OpenPencil-MCP-Servers (HTTP, 127.0.0.1:7600) direkt auf.
// Damit lassen sich große JSX-Dateien rendern, ohne sie in den Aufruf zu kopieren.
//
//   node op.mjs <werkzeug> '<json-argumente>' [--jsx datei.jsx] [--png ausgabe.png] [--voll]
//
// --jsx liest das Argument "jsx" aus einer Datei. --png schreibt ein Bild aus export_image in eine
// Datei statt Base64 auszugeben. --voll gibt lange Antworten ungekürzt aus. Der Server muss ohne Token laufen (OPENPENCIL_MCP_AUTH_TOKEN="").
import { readFileSync, writeFileSync } from 'node:fs'

const [tool, rawArgs = '{}', ...rest] = process.argv.slice(2)
if (!tool) {
  console.error("Aufruf: node op.mjs <werkzeug> '<json>' [--jsx datei] [--png datei]")
  process.exit(2)
}
const opt = (name) => { const i = rest.indexOf(name); return i >= 0 ? rest[i + 1] : undefined }
const args = JSON.parse(rawArgs)
if (opt('--jsx')) args.jsx = readFileSync(opt('--jsx'), 'utf8')

const url = process.env.OPENPENCIL_MCP_URL ?? 'http://127.0.0.1:7600/mcp'
const headers = { 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream' }
async function rpc(body, session) {
  const res = await fetch(url, { method: 'POST', headers: session ? { ...headers, 'Mcp-Session-Id': session } : headers, body: JSON.stringify(body) })
  const text = await res.text()
  // Antwort kommt als JSON oder als SSE-Ereignis ("data: {...}")
  const json = text.startsWith('{') ? JSON.parse(text) : text ? JSON.parse(text.split('\n').filter((l) => l.startsWith('data:')).pop().slice(5)) : null
  return { json, session: res.headers.get('mcp-session-id') ?? session }
}
// Der Server erlaubt höchstens 10 Sitzungen; bei "Too many active MCP sessions" bis zu 2 Minuten warten
let init
for (let i = 0; i < 40; i++) {
  try { init = await rpc({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-03-26', capabilities: {}, clientInfo: { name: 'op.mjs', version: '1' } } }) } catch { init = null }
  if (init?.session && init.json?.result) break
  await new Promise((r) => setTimeout(r, 3000))
}
if (!init?.session) { console.error('OpenPencil-MCP nicht erreichbar oder voll'); process.exit(1) }
const session = init.session
await rpc({ jsonrpc: '2.0', method: 'notifications/initialized' }, session)
const { json } = await rpc({ jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name: tool, arguments: args } }, session)
if (!json || json.error) { console.error(JSON.stringify(json?.error ?? json)); await fetch(url, { method: 'DELETE', headers: { 'Mcp-Session-Id': session } }).catch(() => {}); process.exit(1) }
const content = json.result?.content ?? []
for (const c of content) {
  if (c.type === 'image' && opt('--png')) { writeFileSync(opt('--png'), Buffer.from(c.data, 'base64')); console.log(`Bild: ${opt('--png')}`) }
  else if (c.type === 'text') {
    // export_image liefert das Bild je nach Version als Text-JSON mit Base64
    let parsed
    try { parsed = JSON.parse(c.text) } catch {}
    const b64 = parsed?.data ?? parsed?.images?.[0]?.data ?? parsed?.image
    if (b64 && opt('--png')) { writeFileSync(opt('--png'), Buffer.from(b64, 'base64')); console.log(`Bild: ${opt('--png')}`) }
    else console.log(c.text.length > 4000 && !rest.includes('--voll') ? c.text.slice(0, 4000) + ' …' : c.text)
  } else console.log(JSON.stringify(c).slice(0, 300))
}
// Sitzung schließen, sonst läuft der Server in sein Limit ("Too many active MCP sessions")
await fetch(url, { method: 'DELETE', headers: { 'Mcp-Session-Id': session } }).catch(() => {})
if (json.result?.isError) process.exit(1)
