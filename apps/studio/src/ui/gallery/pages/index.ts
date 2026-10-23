import { createElement } from "react";
import type { ReactNode } from "react";
import { ButtonsPage } from "./ButtonsPage";
import { InputsPage } from "./InputsPage";

export type GalleryPage = { id: string; title: string; element: ReactNode };

/** One page per primitive; later tasks append theirs. */
export const pages: GalleryPage[] = [
  { id: "buttons", title: "Tasten", element: createElement(ButtonsPage) },
  { id: "inputs", title: "Eingaben", element: createElement(InputsPage) },
];
