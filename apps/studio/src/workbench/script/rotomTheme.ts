import { HighlightStyle, syntaxHighlighting, type TagStyle } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { syntax } from "../../ui/tokens/tokens";

type Mode = "hell" | "dunkel";

/** Syntax colours are concrete values from the token table (a highlight style cannot follow the data-theme on its own). */
const highlightOf = (mode: Mode): readonly TagStyle[] => {
  const c = syntax[mode];
  return [
    { tag: tags.keyword, color: c.schluesselwort, fontWeight: "600" },
    { tag: [tags.function(tags.variableName), tags.propertyName], color: c.befehl },
    { tag: tags.variableName, color: "var(--g4-text-primaer)" },
    { tag: tags.number, color: c.zahl },
    { tag: tags.string, color: c.zeichenkette },
    { tag: tags.comment, color: c.kommentar, fontStyle: "italic" },
    { tag: tags.meta, color: c.flag },
    { tag: tags.operator, color: "var(--g4-text-sekundaer)" },
    { tag: [tags.bool, tags.null], color: c.flag },
    { tag: tags.invalid, color: c.fehler },
  ];
};

export const ROTOM_HIGHLIGHT: Record<Mode, readonly TagStyle[]> = {
  hell: highlightOf("hell"),
  dunkel: highlightOf("dunkel"),
};

/** The editor chrome follows the `--g4-*` tokens at runtime; only `dark` differs between the two modes. */
const themeOf = (mode: Mode): Extension => [
  EditorView.theme(
    {
      "&": { backgroundColor: "var(--g4-flaeche-blatt)", color: "var(--g4-text-primaer)" },
      ".cm-content": { caretColor: "var(--g4-akzent-grund)" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--g4-akzent-grund)" },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
        backgroundColor: "var(--g4-auswahl-flaeche)",
      },
      ".cm-gutters": {
        backgroundColor: "var(--g4-flaeche-blatt)",
        color: "var(--g4-text-sekundaer)",
        border: "none",
        borderRight: "1px solid var(--g4-linie-standard)",
      },
      ".cm-activeLine": { backgroundColor: "var(--g4-flaeche-schau)" },
      ".cm-activeLineGutter": { backgroundColor: "var(--g4-flaeche-schau)", color: "var(--g4-text-primaer)" },
      ".cm-lintRange-error": { backgroundImage: "none", textDecoration: "underline wavy var(--g4-status-fehler)" },
      ".cm-lintRange-warning": { backgroundImage: "none", textDecoration: "underline wavy var(--g4-status-warnung)" },
      ".cm-lintRange-info, .cm-lintRange-hint": { backgroundImage: "none", textDecoration: "underline wavy var(--g4-status-erfolg)" },
      ".cm-diagnostic-error": { borderLeftColor: "var(--g4-status-fehler)" },
      ".cm-diagnostic-warning": { borderLeftColor: "var(--g4-status-warnung)" },
      ".cm-diagnostic-info, .cm-diagnostic-hint": { borderLeftColor: "var(--g4-status-erfolg)" },
      ".cm-tooltip": {
        backgroundColor: "var(--g4-flaeche-blatt)",
        color: "var(--g4-text-primaer)",
        border: "1px solid var(--g4-linie-standard)",
        borderRadius: "var(--g4-radius-feld)",
        boxShadow: "var(--g4-schatten-schwebend)",
      },
      ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
        backgroundColor: "var(--g4-auswahl-flaeche)",
        color: "var(--g4-text-primaer)",
      },
      ".cm-scroller": { fontFamily: "var(--g4-schrift-familie-code)" },
    },
    { dark: mode === "dunkel" },
  ),
  syntaxHighlighting(HighlightStyle.define([...ROTOM_HIGHLIGHT[mode]])),
];

/** One extension per mode, as module constants: the editor reconfigures its theme slot only when the identity changes. */
export const ROTOM_THEME: Record<Mode, Extension> = {
  hell: themeOf("hell"),
  dunkel: themeOf("dunkel"),
};
