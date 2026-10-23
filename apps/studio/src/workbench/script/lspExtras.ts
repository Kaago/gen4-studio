import { LSPPlugin, type LSPClient, type LSPClientExtension } from "@codemirror/lsp-client";
import { StateEffect, StateField, type Extension } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, WidgetType, type DecorationSet, type ViewUpdate } from "@codemirror/view";

// @codemirror/lsp-client has no documentSymbol, inlayHint or codeLens support (spec §2.6). The LSP shapes used
// here are declared locally: vscode-languageserver-protocol is only a transitive dependency of the client.
interface LspPosition {
  line: number;
  character: number;
}
interface LspRange {
  start: LspPosition;
  end: LspPosition;
}
interface LspSymbol {
  name: string;
  detail?: string;
  range?: LspRange;
  selectionRange?: LspRange;
  /** SymbolInformation (flat) instead of DocumentSymbol. */
  location?: { range: LspRange };
  children?: LspSymbol[];
}
interface LspInlayHint {
  position: LspPosition;
  label: string | { value: string }[];
  paddingLeft?: boolean;
  paddingRight?: boolean;
}
interface LspCodeLens {
  range: LspRange;
  command?: { title: string };
  data?: unknown;
}

/** Wait after the last edit before inlay hints and code lenses are asked for again. */
const REFRESH_DELAY_MS = 300;

/** Pass this in `LSPClient`'s `extensions` (beside `languageServerExtensions()`) so the server knows the extras. */
export const lspExtras: LSPClientExtension = {
  clientCapabilities: {
    textDocument: {
      documentSymbol: { hierarchicalDocumentSymbolSupport: true },
      inlayHint: { dynamicRegistration: false },
      codeLens: { dynamicRegistration: false },
    },
  },
};

/**
 * The symbols of `uri` (for the outline menu), flattened, in server order. `line` is 1-based, like the problem
 * lines and `Text.line`. Rejects like any client request (not connected, timeout, unsupported).
 */
export async function documentSymbols(client: LSPClient, uri: string): Promise<{ name: string; detail?: string; line: number }[]> {
  client.sync();
  const result = await client.request<{ textDocument: { uri: string } }, LspSymbol[] | null>("textDocument/documentSymbol", {
    textDocument: { uri },
  });
  const out: { name: string; detail?: string; line: number }[] = [];
  const visit = (symbol: LspSymbol) => {
    const range = symbol.selectionRange ?? symbol.range ?? symbol.location?.range;
    if (range) out.push({ name: symbol.name, detail: symbol.detail, line: range.start.line + 1 });
    symbol.children?.forEach(visit);
  };
  result?.forEach(visit);
  return out;
}

/**
 * A view plugin that calls `load` once after creation and again `REFRESH_DELAY_MS` after the last edit, and
 * dispatches the effect it returns, unless the view was destroyed or a newer load started meanwhile. Does nothing
 * for an editor without the LSP plugin. A failed load (server without the feature, timeout, dead server) keeps the
 * previous decorations: hints and lenses are only a display.
 */
function refreshing(load: (view: EditorView, plugin: LSPPlugin) => Promise<StateEffect<unknown> | null>): Extension {
  return ViewPlugin.fromClass(
    class {
      private timer: ReturnType<typeof setTimeout> | undefined;
      private generation = 0;
      private destroyed = false;

      constructor(private readonly view: EditorView) {
        this.schedule();
      }

      update(update: ViewUpdate) {
        if (update.docChanged) this.schedule();
      }

      destroy() {
        this.destroyed = true;
        clearTimeout(this.timer);
      }

      private schedule() {
        clearTimeout(this.timer);
        this.timer = setTimeout(() => void this.run(), REFRESH_DELAY_MS);
      }

      private async run() {
        const plugin = LSPPlugin.get(this.view);
        if (!plugin) return;
        const generation = ++this.generation;
        try {
          const effect = await load(this.view, plugin);
          if (effect && !this.destroyed && generation === this.generation) this.view.dispatch({ effects: effect });
        } catch {
          // See above: keep what is shown.
        }
      }
    },
  );
}

/** A decoration set that follows the document and is replaced by its effect. */
function decorationField(effect: ReturnType<typeof StateEffect.define<DecorationSet>>): Extension {
  return StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update(value, tr) {
      let next = value.map(tr.changes);
      for (const e of tr.effects) if (e.is(effect)) next = e.value;
      return next;
    },
    provide: (field) => EditorView.decorations.from(field),
  });
}

/** Runs `request` against a synchronized document and hands `build` the mapping from request-time positions to the current ones. */
async function withServer<T>(
  plugin: LSPPlugin,
  capability: string,
  request: () => Promise<T | null>,
  build: (result: T, map: (pos: LspPosition, assoc: -1 | 1) => number) => DecorationSet,
): Promise<DecorationSet | null> {
  const { client, uri } = plugin;
  await client.initializing;
  if (!(client.serverCapabilities as Record<string, unknown> | null)?.[capability]) return null;
  return client.withMapping(async (mapping) => {
    client.sync();
    const result = await request();
    if (result === null) return null;
    const length = plugin.view.state.doc.length;
    return build(result, (pos, assoc) => Math.min(mapping.mapPosition(uri, pos, assoc), length));
  });
}

class InlayHintWidget extends WidgetType {
  constructor(
    private readonly label: string,
    private readonly paddingLeft: boolean,
    private readonly paddingRight: boolean,
  ) {
    super();
  }

  eq(other: InlayHintWidget) {
    return other.label === this.label && other.paddingLeft === this.paddingLeft && other.paddingRight === this.paddingRight;
  }

  toDOM() {
    const el = document.createElement("span");
    el.className = "cm-inlay-hint";
    el.textContent = this.label;
    if (this.paddingLeft) el.style.marginLeft = "0.3em";
    if (this.paddingRight) el.style.marginRight = "0.3em";
    return el;
  }
}

const setInlayHints = StateEffect.define<DecorationSet>();

/** Inlay hints (`textDocument/inlayHint`) for the whole document as widgets, refreshed 300 ms after the last edit. */
export const inlayHints: Extension = [
  decorationField(setInlayHints),
  refreshing(async (view, plugin) => {
    const decorations = await withServer<LspInlayHint[]>(
      plugin,
      "inlayHintProvider",
      () =>
        plugin.client.request("textDocument/inlayHint", {
          textDocument: { uri: plugin.uri },
          range: { start: { line: 0, character: 0 }, end: plugin.toPosition(view.state.doc.length) },
        }),
      (hints, map) =>
        Decoration.set(
          hints.map((hint) => {
            const label = typeof hint.label === "string" ? hint.label : hint.label.map((part) => part.value).join("");
            const widget = new InlayHintWidget(label, !!hint.paddingLeft, !!hint.paddingRight);
            return Decoration.widget({ widget, side: 1 }).range(map(hint.position, 1));
          }),
          true,
        ),
    );
    return decorations && setInlayHints.of(decorations);
  }),
  EditorView.baseTheme({
    ".cm-inlay-hint": { opacity: 0.6, fontSize: "90%", fontStyle: "italic", userSelect: "none" },
  }),
];

class CodeLensWidget extends WidgetType {
  constructor(private readonly titles: string[]) {
    super();
  }

  eq(other: CodeLensWidget) {
    return other.titles.join("\0") === this.titles.join("\0");
  }

  toDOM() {
    const el = document.createElement("div");
    el.className = "cm-code-lens";
    el.textContent = this.titles.join(" | ");
    return el;
  }
}

const setCodeLenses = StateEffect.define<DecorationSet>();

/** Code lenses (`textDocument/codeLens`, unresolved ones resolved) as text lines above their line; display only. */
export const codeLenses: Extension = [
  decorationField(setCodeLenses),
  refreshing(async (view, plugin) => {
    const { client } = plugin;
    const decorations = await withServer<LspCodeLens[]>(
      plugin,
      "codeLensProvider",
      async () => {
        const lenses = await client.request<unknown, LspCodeLens[] | null>("textDocument/codeLens", { textDocument: { uri: plugin.uri } });
        if (!lenses) return null;
        return Promise.all(
          lenses.map((lens) =>
            lens.command ? lens : client.request<LspCodeLens, LspCodeLens | null>("codeLens/resolve", lens).then((r) => r ?? lens, () => lens),
          ),
        );
      },
      (lenses, map) => {
        const byLine = new Map<number, string[]>();
        for (const lens of lenses) {
          if (!lens.command) continue;
          const line = view.state.doc.lineAt(map(lens.range.start, -1)).from;
          byLine.set(line, [...(byLine.get(line) ?? []), lens.command.title]);
        }
        return Decoration.set(
          [...byLine].map(([from, titles]) => Decoration.widget({ widget: new CodeLensWidget(titles), block: true, side: -1 }).range(from)),
          true,
        );
      },
    );
    return decorations && setCodeLenses.of(decorations);
  }),
  EditorView.baseTheme({
    ".cm-code-lens": { opacity: 0.6, fontSize: "85%", padding: "0 6px", userSelect: "none" },
  }),
];
