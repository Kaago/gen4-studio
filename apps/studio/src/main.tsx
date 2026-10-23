import React from "react";
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
import { inTauri } from "./platform/transport";
import { Gallery } from "./ui/gallery/Gallery";
import App from "./App";

applyTheme(readTheme());
applyDensity(readDensity());

const showGallery = !inTauri() && new URLSearchParams(location.search).has("gallery");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{showGallery ? <Gallery /> : <App />}</React.StrictMode>,
);
