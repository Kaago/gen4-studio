import { TERMS } from "../terms";

interface DoneProps {
  /** What was made, e.g. "Gebiet angelegt". */
  title: string;
  lines: string[];
  next: string[];
}

/** The result step of a wizard that created everything: what it made and what to do next (the frame adds "Zeigen" and "Hier spielen"). */
export function DoneResult({ title, lines, next }: DoneProps) {
  return (
    <div>
      <div style={{ fontWeight: 600, marginBottom: 6 }}>{title}</div>
      {lines.map((line) => (
        <p key={line} style={{ margin: "4px 0" }}>
          {line}
        </p>
      ))}
      <div style={{ fontWeight: 600, margin: "10px 0 4px" }}>Als Nächstes</div>
      <ul style={{ margin: 0, paddingLeft: 18 }}>
        {next.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
    </div>
  );
}

/** The result step of a wizard that failed after it had created members: what the backend says stayed behind. */
export function PartialResult({ message }: { message: string }) {
  return (
    <div>
      <div style={{ fontWeight: 600, marginBottom: 6 }}>Anlegen unterbrochen</div>
      <p style={{ margin: "4px 0" }}>{message}</p>
      <p style={{ margin: "4px 0" }}>
        Was schon angelegt ist, bleibt bestehen und lässt sich nicht rückgängig machen. Den Rest im {TERMS.matrix.label}-Tab und im {TERMS.header.label}-Editor von Hand fertigstellen.
      </p>
    </div>
  );
}
