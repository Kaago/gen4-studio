// First, before anything that could pull in the app: i18next must be initialized when the app modules are evaluated.
import "./i18n/boot";
import React, { lazy, Suspense } from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/atkinson-hyperlegible-next";
import "@fontsource/atkinson-hyperlegible-mono/400.css";
import "@fontsource/zen-maru-gothic/latin-700.css";
import "@fontsource/zen-maru-gothic/latin-900.css";
import "@fontsource/pixelify-sans/400.css";
import "./ui/tokens/tokens.css";
import "./ui/tokens/base.css";
import "dockview/dist/styles/dockview.css";
import "./ui/tokens/dockview-theme.css";
import { applyDensity, applyTheme, readDensity, readTheme } from "./ui/tokens/theme";
import { startLanguage } from "./i18n";
import { inTauri } from "./platform/transport";
import App from "./App";

// Only the browser mode with ?gallery loads the gallery; it stays out of the app's own chunk.
const Gallery = lazy(() => import("./ui/gallery/Gallery").then((m) => ({ default: m.Gallery })));

/**
 * Styling first (it must not wait for the backend), then the language (the first render must already speak it, and the backend
 * must know it; with „System“ inside Tauri that is the OS language the backend reports), then the render. i18next itself is
 * initialized earlier, by `./i18n/boot`, with the navigator's guess.
 */
async function boot(): Promise<void> {
  applyTheme(readTheme());
  applyDensity(readDensity());
  await startLanguage();

  const showGallery = !inTauri() && new URLSearchParams(location.search).has("gallery");

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      {showGallery ? (
        <Suspense fallback={null}>
          <Gallery />
        </Suspense>
      ) : (
        <App />
      )}
    </React.StrictMode>,
  );
}

boot().catch((e) => console.error("boot failed", e));
