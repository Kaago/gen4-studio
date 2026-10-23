import { StreamLanguage, StringStream, type StreamParser } from "@codemirror/language";
import { tags } from "@lezer/highlight";

/** Words that open a definition; the name that follows is the definition, not a command. */
const DEFINERS = new Set(["script", "function", "action"]);
const KEYWORDS = new Set([...DEFINERS, "if", "else", "while", "match"]);

interface RotomState {
  /** The previous token was `script`, `function` or `action`. */
  afterDefiner: boolean;
}

/**
 * Token styles are CodeMirror legacy token names (see `tokenTable` for `command`). Commands are the
 * capitalised identifiers (`Message`, `End`), `#N` is a script label, `#include` / `#define` are directives.
 */
const parser: StreamParser<RotomState> = {
  name: "rotom",
  startState: () => ({ afterDefiner: false }),
  token(stream, state) {
    if (stream.eatSpace()) return null;
    const afterDefiner = state.afterDefiner;
    state.afterDefiner = false;

    if (stream.match("//")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.eat('"')) {
      while (!stream.eol()) {
        const ch = stream.next();
        if (ch === "\\") stream.next();
        else if (ch === '"') break;
      }
      return "string";
    }
    if (stream.match(/^#(?:include|define)\b/)) return "meta";
    if (stream.match(/^#\d+/)) return "number";
    if (stream.match(/^0[xX][0-9a-fA-F]+/) || stream.match(/^\d+/)) return "number";
    if (stream.match(/^[A-Za-z_][A-Za-z0-9_]*/)) {
      const word = stream.current();
      if (KEYWORDS.has(word)) {
        state.afterDefiner = DEFINERS.has(word);
        return "keyword";
      }
      if (afterDefiner) return "variableName";
      return /^[A-Z]/.test(word) ? "command" : "variableName";
    }
    if (stream.match(/^(?:[=!<>]=|&&|\|\||[=<>+\-*/!])/)) return "operator";
    stream.next();
    return null;
  },
  languageData: { commentTokens: { line: "//" } },
  tokenTable: { command: tags.function(tags.variableName) },
};

export const rotomLanguage: StreamLanguage<RotomState> = StreamLanguage.define(parser);

/** The styled tokens of one line (unstyled ones such as `:` get style ""; whitespace is left out). For tests. */
export function tokenize(line: string): { text: string; style: string }[] {
  const stream = new StringStream(line, 4, 4);
  const state = parser.startState!(4);
  const out: { text: string; style: string }[] = [];
  while (!stream.eol()) {
    stream.start = stream.pos;
    const style = parser.token(stream, state);
    if (stream.pos === stream.start) throw new Error(`rotom tokenizer made no progress at ${stream.pos}`);
    const text = stream.current();
    if (text.trim() !== "") out.push({ text, style: style ?? "" });
  }
  return out;
}
