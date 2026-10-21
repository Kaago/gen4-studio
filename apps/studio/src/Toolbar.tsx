import type { CSSProperties, ReactNode } from "react";

export type Message = { kind: "info" | "error"; text: string };

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

/** Non-modal message bar at the bottom: errors and results of save/build/run. */
export function StatusBar({ message, onDismiss }: { message: Message | null; onDismiss: () => void }) {
  if (!message) return null;
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
      <span style={{ flex: 1, minWidth: 0, overflowWrap: "anywhere" }}>{message.text}</span>
      <button onClick={onDismiss}>Schließen</button>
    </div>
  );
}
