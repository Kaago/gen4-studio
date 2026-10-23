import { useState } from "react";
import { ToastHost } from "../../frame/ToastHost";
import { toast } from "../../frame/toasts";
import { Button, Dialog, IconButton, Menu, Popover } from "../../primitives";
import styles from "../Gallery.module.css";

export function OverlayPage() {
  const [dialog, setDialog] = useState<"closed" | "question" | "form">("closed");
  const [log, setLog] = useState("Noch nichts gewählt.");
  const progress = () => {
    let step = 0;
    toast.progress("gallery-load", "Lade Karten … 0 von 3");
    const timer = setInterval(() => {
      step += 1;
      if (step > 3) {
        clearInterval(timer);
        toast.progress("gallery-load", null);
        toast.info("Alle 3 Karten geladen.");
      } else toast.progress("gallery-load", `Lade Karten … ${step} von 3`);
    }, 900);
  };

  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Menü</h2>
        <div className={styles.row}>
          <Menu
            trigger="Verlauf"
            items={[
              { label: "Tür verschoben", icon: "door", onSelect: () => setLog("Tür verschoben") },
              { label: "Figur angelegt", icon: "figure", onSelect: () => setLog("Figur angelegt") },
              "separator",
              { label: "Alles zurücknehmen", icon: "undo", onSelect: () => setLog("Alles zurücknehmen"), disabled: true },
            ]}
          />
          <span className={styles.note}>{log}</span>
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Kurzinfo</h2>
        <p className={styles.note}>Erscheint nach 400 ms Hover oder beim Fokus per Tastatur.</p>
        <div className={styles.row}>
          <IconButton label="Speichern" icon="save" shortcut="⌘S" onClick={() => {}} />
          <IconButton label="Suchen" icon="search" onClick={() => {}} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Dialog</h2>
        <div className={styles.row}>
          <Button kind="secondary" onClick={() => setDialog("question")}>Tür löschen …</Button>
          <Button kind="secondary" onClick={() => setDialog("form")}>Gebiet anlegen …</Button>
        </div>
        <Dialog
          open={dialog === "question"}
          onOpenChange={(open) => !open && setDialog("closed")}
          title="Tür wirklich löschen?"
          description="Die Tür verschwindet aus dem Ort. Das lässt sich nicht zurücknehmen."
          actions={[
            { label: "Behalten", onClick: () => setDialog("closed") },
            { label: "Tür löschen", kind: "danger", onClick: () => setDialog("closed") },
          ]}
        />
        <Dialog
          open={dialog === "form"}
          onOpenChange={(open) => !open && setDialog("closed")}
          title="Gebiet anlegen"
          actions={[
            { label: "Abbrechen", kind: "quiet", onClick: () => setDialog("closed") },
            { label: "Gebiet anlegen", kind: "primary", onClick: () => setDialog("closed") },
          ]}
        >
          <p className={styles.note}>Tab bleibt im Dialog, Escape schließt ihn, der Fokus kehrt zur Taste zurück.</p>
        </Dialog>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Popover</h2>
        <div className={styles.row}>
          <Popover trigger="Farbe wählen">
            <p className={styles.note}>Escape oder ein Klick daneben schließt das Fenster.</p>
          </Popover>
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Meldungen (Toasts)</h2>
        <p className={styles.note}>Info verschwindet nach 5 s (Hover hält an), ein Fehler bleibt, Fortschritt ersetzt sich selbst. Höchstens vier.</p>
        <div className={styles.row}>
          <Button kind="secondary" onClick={() => toast.info("Gespeichert.")}>Info</Button>
          <Button kind="secondary" onClick={() => toast.error("Die Datei fehlt. Wähle den Projektordner neu.")}>Fehler</Button>
          <Button
            kind="secondary"
            onClick={() =>
              toast.error("1 Fehler in Scripts, Build abgebrochen.", { actions: [{ label: "Probleme zeigen", onClick: () => setLog("Probleme gezeigt") }] })
            }
          >
            Fehler mit Taste
          </Button>
          <Button kind="secondary" onClick={progress}>Fortschritt</Button>
        </div>
      </section>
      <ToastHost />
    </div>
  );
}
