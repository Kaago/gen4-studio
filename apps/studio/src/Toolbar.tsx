import type { CSSProperties, ReactNode } from "react";
import { countsText } from "./workbench/problems/problems";
import type { ProblemCounts } from "./workbench/problems/problems";

export interface MessageAction {
  label: string;
  run: () => void;
}

export type Message = { kind: "info" | "error"; text: string; actions?: MessageAction[] };

const bar: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  padding: "6px 12px",
  borderBottom: "1px solid #ccc",
  fontFamily: "system-ui, sans-serif",
};

interface ToolbarProps {
  projectOpen: boolean;
  busy: boolean;
  dirtyCount: number;
  canRun: boolean;
  onOpen: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onSave: () => void;
  onBuild: () => void;
  onRun: () => void;
  onPickEmulator: () => void;
}

export function Toolbar(p: ToolbarProps) {
  const edit = !p.projectOpen || p.busy;
  const button = (label: string, onClick: () => void, disabled: boolean, title?: string): ReactNode => (
    <button onClick={onClick} disabled={disabled} title={title}>
      {label}
    </button>
  );
  return (
    <div style={bar}>
      {button("Projekt öffnen", p.onOpen, p.busy)}
      {button("Undo", p.onUndo, edit, "Cmd/Ctrl+Z")}
      {button("Redo", p.onRedo, edit, "Shift+Cmd/Ctrl+Z")}
      {button("Speichern", p.onSave, edit, "Cmd/Ctrl+S")}
      {button("Bauen", p.onBuild, edit, "Speichert zuerst und baut die ROM")}
      {button("Starten", p.onRun, edit || !p.canRun, p.canRun ? "ROM im Emulator starten" : "Zuerst bauen")}
      {button("Emulator wählen", p.onPickEmulator, p.busy)}
      <span style={{ marginLeft: 8, color: p.dirtyCount > 0 ? "#b45309" : "#6b7280" }}>
        {p.dirtyCount > 0 ? `● ungespeichert (${p.dirtyCount})` : p.projectOpen ? "gespeichert" : ""}
      </span>
    </div>
  );
}

/** Non-modal message bar at the bottom: errors and results of save/build/run, and the problem count ("2 Fehler, 1 Warnung"). */
export function StatusBar({ message, problems, onDismiss }: { message: Message | null; problems?: ProblemCounts; onDismiss: () => void }) {
  const summary = problems ? countsText(problems) : "";
  if (!message) {
    if (!summary) return null;
    return (
      <div role="status" style={{ ...bar, borderTop: "1px solid #ccc", borderBottom: "none", color: problems!.errors > 0 ? "#b91c1c" : "#b45309" }}>
        {summary}
      </div>
    );
  }
  const error = message.kind === "error";
  return (
    <div
      role={error ? "alert" : "status"}
      style={{
        ...bar,
        borderTop: "1px solid #ccc",
        borderBottom: "none",
        color: error ? "#fff" : "#111",
        background: error ? "#b91c1c" : "#e5e7eb",
      }}
    >
      <span
        title={message.text}
        style={{ flex: 1, minWidth: 0, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}
      >
        {message.text}
      </span>
      {summary && <span style={{ flexShrink: 0 }}>{summary}</span>}
      {message.actions?.map((a) => (
        <button key={a.label} onClick={a.run} style={{ flexShrink: 0 }}>
          {a.label}
        </button>
      ))}
      <button onClick={onDismiss} style={{ flexShrink: 0 }}>
        Schließen
      </button>
    </div>
  );
}
