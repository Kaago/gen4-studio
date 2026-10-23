import { createElement } from "react";
import type { ReactNode } from "react";
import { ButtonsPage } from "./ButtonsPage";
import { ContentPage } from "./ContentPage";
import { InputsPage } from "./InputsPage";
import { OverlayPage } from "./OverlayPage";

export type GalleryPage = { id: string; title: string; element: ReactNode };

/** One page per primitive group; later tasks append theirs. */
export const pages: GalleryPage[] = [
  { id: "buttons", title: "Tasten", element: createElement(ButtonsPage) },
  { id: "inputs", title: "Eingaben", element: createElement(InputsPage) },
  { id: "content", title: "Inhalt und Status", element: createElement(ContentPage) },
  { id: "overlay", title: "Schwebendes", element: createElement(OverlayPage) },
];
