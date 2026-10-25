import { createContext } from "react";
import { checkProjectDest, createProject, defaultProjectParent, inspectRom } from "../api";

/** The backend calls of "Neues Spiel aus ROM". */
export type StartService = {
  inspectRom: typeof inspectRom;
  defaultProjectParent: typeof defaultProjectParent;
  checkProjectDest: typeof checkProjectDest;
  createProject: typeof createProject;
};

/** The real ones by default; the gallery shows the wizard with example data through its own. */
export const StartServiceContext = createContext<StartService>({ inspectRom, defaultProjectParent, checkProjectDest, createProject });
