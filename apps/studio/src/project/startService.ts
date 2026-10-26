import { createContext } from "react";
import { checkProjectDest, createProject, defaultProjectParent, inspectRom, projectPreviewGet, projectPreviewSet } from "../api";

/** The backend calls of the area Start: "Neues Spiel aus ROM" and the preview pictures of the recent projects. */
export type StartService = {
  inspectRom: typeof inspectRom;
  defaultProjectParent: typeof defaultProjectParent;
  checkProjectDest: typeof checkProjectDest;
  createProject: typeof createProject;
  projectPreviewGet: typeof projectPreviewGet;
  projectPreviewSet: typeof projectPreviewSet;
};

/** The real ones by default; the gallery shows the wizard with example data through its own. */
export const StartServiceContext = createContext<StartService>({
  inspectRom,
  defaultProjectParent,
  checkProjectDest,
  createProject,
  projectPreviewGet,
  projectPreviewSet,
});
