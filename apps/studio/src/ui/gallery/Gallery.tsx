import { useState } from "react";
import { Button, Segmented } from "../primitives";
import { applyDensity, applyTheme, readDensity, readTheme } from "../tokens/theme";
import type { Density, Theme } from "../tokens/theme";
import { pages } from "./pages";
import styles from "./Gallery.module.css";

const themes = [
  { value: "hell", label: "Hell", icon: "sun" },
  { value: "dunkel", label: "Dunkel", icon: "moon" },
  { value: "system", label: "System" },
] as const;
const densities = [
  { value: "komfort", label: "Komfort" },
  { value: "kompakt", label: "Kompakt" },
] as const;

/** Browser-mode design system gallery (`?gallery`): one page per primitive, theme and density switches on top. */
export function Gallery() {
  const [theme, setTheme] = useState<Theme>(readTheme);
  const [density, setDensity] = useState<Density>(readDensity);
  const [pageId, setPageId] = useState(pages[0]?.id ?? "");
  const page = pages.find((p) => p.id === pageId) ?? pages[0];

  const chooseTheme = (t: Theme) => {
    applyTheme(t);
    setTheme(t);
  };
  const chooseDensity = (d: Density) => {
    applyDensity(d);
    setDensity(d);
  };

  return (
    <div className={styles.gallery}>
      <header className={styles.head}>
        <h1 className={styles.title}>Galerie</h1>
        <div className={styles.switches} role="group" aria-label="Darstellung">
          <Segmented ariaLabel="Thema" options={themes} value={theme} onChange={chooseTheme} />
          <Segmented ariaLabel="Dichte" options={densities} value={density} onChange={chooseDensity} />
        </div>
      </header>
      <nav className={styles.nav} aria-label="Seiten">
        {pages.map((p) => (
          <Button key={p.id} kind="quiet" aria-current={p.id === page?.id ? "page" : undefined} onClick={() => setPageId(p.id)}>
            {p.title}
          </Button>
        ))}
      </nav>
      <main className={styles.page}>{page?.element}</main>
    </div>
  );
}
