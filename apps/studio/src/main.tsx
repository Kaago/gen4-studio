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
import { applyLanguage, init, readLanguage } from "./i18n";
import { inTauri } from "./platform/transport";
import App from "./App";

// Only the browser mode with ?gallery loads the gallery; it stays out of the app's own chunk.
const Gallery = lazy(() => import("./ui/gallery/Gallery").then((m) => ({ default: m.Gallery })));

/** Language first (the first render must already speak it, and the backend must know it), then theme, density and the render. */
async function boot(): Promise<void> {
  init(readLanguage());
  await applyLanguage(readLanguage());

  applyTheme(readTheme());
  applyDensity(readDensity());

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

void boot();
