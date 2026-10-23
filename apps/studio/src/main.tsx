import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/atkinson-hyperlegible-next";
import "@fontsource/atkinson-hyperlegible-mono/400.css";
import "@fontsource/zen-maru-gothic/700.css";
import "@fontsource/zen-maru-gothic/900.css";
import "@fontsource/pixelify-sans/400.css";
import "./ui/tokens/tokens.css";
import "./ui/tokens/base.css";
import "dockview/dist/styles/dockview.css";
import "./ui/tokens/dockview-theme.css";
import { applyDensity, applyTheme, readDensity, readTheme } from "./ui/tokens/theme";
import App from "./App";

applyTheme(readTheme());
applyDensity(readDensity());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
