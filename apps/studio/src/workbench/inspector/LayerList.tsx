import { useEffect, useId, useState } from "react";
import type { JSX } from "react";
import { worldLayers } from "../../api.world";
import type { LayerOp, WorldJson } from "../../api.world";
import { Button, IconButton, Section, TextField } from "../../ui/primitives";
import { useSectionOpen } from "./foldState";
import styles from "./LayerList.module.css";

/**
 * The layers of a world map while painting (section „Ebenen“): one row per layer with its name and role („Boden“, „Überlagerung“), the
 * radio „Aktiv“ (the layer the brush paints), the eye (shown or hidden in the scene only, never saved: `onToggle`, no kernel call) and
 * „Entfernen“ for overlays; below „Ebene hinzufügen“ with a name (proposed: „Ebene <n>“). Adding and removing go through
 * `world_layers`, one undo step each; a refusal goes to `onError`, a change to `onChanged` (scene, world and history catch up).
 */
export function LayerList({
  map,
  world,
  active,
  hidden,
  onActive,
  onToggle,
  onError,
  onChanged,
  edit = (op) => worldLayers(map, op),
}: {
  map: number;
  world: WorldJson;
  active: number;
  hidden: ReadonlySet<number>;
  onActive: (layer: number) => void;
  onToggle: (layer: number) => void;
  onError: (e: unknown) => void;
  /** A layer was added or removed. */
  onChanged?: (map: number) => void;
  /** Where adding and removing go (the gallery writes nothing). */
  edit?: (op: LayerOp) => Promise<unknown>;
}): JSX.Element {
  const [open, toggle] = useSectionOpen("matrixCell:layers", true);
  const group = useId();
  const count = world.layers.length;
  const [name, setName] = useState(`Ebene ${count}`);
  // Another number of layers proposes the next number again.
  useEffect(() => setName(`Ebene ${count}`), [count]);
  const [busy, setBusy] = useState(false);
  const run = async (op: LayerOp) => {
    setBusy(true);
    try {
      await edit(op);
      onChanged?.(map);
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div data-paint-layers="">
      <Section title="Ebenen" open={open} onToggle={toggle}>
        <div className={styles.body}>
          <ul className={styles.rows}>
            {world.layers.map((layer, i) => {
              const isHidden = hidden.has(i);
              return (
                <li key={i} className={styles.row} data-layer-row={i}>
                  <input type="radio" className={styles.radio} name={group} aria-label="Aktiv" checked={active === i} onChange={() => onActive(i)} />
                  <span className={styles.name}>
                    {layer.name} · {layer.role === "ground" ? "Boden" : "Überlagerung"}
                  </span>
                  <IconButton label={isHidden ? "Zeigen" : "Verbergen"} icon={isHidden ? "eyeSlash" : "eye"} pressed={isHidden} onClick={() => onToggle(i)} />
                  {layer.role !== "ground" && (
                    <Button kind="quiet" size="s" disabled={busy} onClick={() => void run({ remove: { layer: i } })}>
                      Entfernen
                    </Button>
                  )}
                </li>
              );
            })}
          </ul>
          <div className={styles.add}>
            <div className={styles.grow}>
              <TextField label="Name der neuen Ebene" value={name} onChange={setName} onCommit={setName} />
            </div>
            <Button kind="secondary" size="s" disabled={busy} onClick={() => void run({ add: { name: name.trim() } })}>
              Ebene hinzufügen
            </Button>
          </div>
        </div>
      </Section>
    </div>
  );
}
