import { useId } from "react";
import { Icon, Popover } from "../ui/primitives";
import { legendEntries } from "../world/legend";
import styles from "./Legend.module.css";

const ENTRIES = legendEntries();

function Swatch({ color }: { color: string }) {
  return <span className={styles.swatch} style={{ background: color }} /* dyn */ aria-hidden="true" />;
}

/** The legend of the scene at the bottom left of the 3D view: every colour with its name (never the colour alone). */
export function Legend() {
  const id = useId();
  return (
    <div className={styles.legend}>
      <Popover
        trigger={
          <>
            <Icon name="legend" />
            Legende
          </>
        }
      >
        <section className={styles.group} aria-labelledby={`${id}-terrain`}>
          <h3 id={`${id}-terrain`} className={styles.heading}>
            Gelände
          </h3>
          <ul className={styles.list}>
            {ENTRIES.terrain.map((t) => (
              <li key={t.key} className={styles.entry}>
                <Swatch color={t.color} />
                {t.label}
              </li>
            ))}
          </ul>
        </section>
        <section className={styles.group} aria-labelledby={`${id}-events`}>
          <h3 id={`${id}-events`} className={styles.heading}>
            Ereignisse
          </h3>
          <ul className={styles.list}>
            {ENTRIES.events.map((e) => (
              <li key={e.kind} className={styles.entry}>
                <Swatch color={e.color} />
                <Icon name={e.icon} />
                {e.label}
              </li>
            ))}
          </ul>
        </section>
      </Popover>
    </div>
  );
}
