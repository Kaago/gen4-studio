import { createElement } from "react";
import type { ReactNode } from "react";
import { ButtonsPage } from "./ButtonsPage";
import { ContentPage } from "./ContentPage";
import { DataPage } from "./DataPage";
import { FramePage } from "./FramePage";
import { InputsPage } from "./InputsPage";
import { OverlayPage } from "./OverlayPage";
import { PickerPage } from "./PickerPage";
import { ProjectPage } from "./ProjectPage";
import { ScreenPage } from "./ScreenPage";
import { WorldPage } from "./WorldPage";

export type GalleryPage = { id: string; title: string; element: ReactNode };

/** One page per primitive group; later tasks append theirs. */
export const pages: GalleryPage[] = [
  { id: "buttons", title: "Tasten", element: createElement(ButtonsPage) },
  { id: "inputs", title: "Eingaben", element: createElement(InputsPage) },
  { id: "content", title: "Inhalt und Status", element: createElement(ContentPage) },
  { id: "overlay", title: "Schwebendes", element: createElement(OverlayPage) },
  { id: "picker", title: "Auswahl", element: createElement(PickerPage) },
  { id: "screen", title: "Bildschirm", element: createElement(ScreenPage) },
  { id: "data", title: "Daten", element: createElement(DataPage) },
  { id: "frame", title: "Rahmen", element: createElement(FramePage) },
  { id: "world", title: "Welt", element: createElement(WorldPage) },
  { id: "project", title: "Projektstart", element: createElement(ProjectPage) },
];
