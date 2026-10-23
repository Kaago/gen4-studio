import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { applyDensity, applyTheme, effectiveTheme, onThemeChange, readDensity } from "../tokens/theme";
import type { Density } from "../tokens/theme";
import { clampHand, readShellState, rememberRecent, writeShellState } from "./shellState";
import type { AreaId, Depth, RecentProject, ShellState, Tool } from "./shellState";

export interface ShellApi {
  state: ShellState;
  setArea: (area: AreaId) => void;
  setListOpen: (area: AreaId, open: boolean) => void;
  setHandHeight: (area: AreaId, height: number) => void;
  toggleHandCollapsed: (area: AreaId) => void;
  setDepth: (depth: Depth) => void;
  /** The tool of the scene; not remembered between runs. */
  tool: Tool;
  setTool: (tool: Tool) => void;
  /** Puts a project first in the recent list (and drops an earlier entry of the same path). */
  remember: (project: RecentProject) => void;
  forget: (path: string) => void;
  /** The theme that is shown; the title bar button switches hell and dunkel ("System" is a setting). */
  theme: "hell" | "dunkel";
  toggleTheme: () => void;
  density: Density;
  toggleDensity: () => void;
}

const ShellContext = createContext<ShellApi | null>(null);

/**
 * The frame's own state, apart from `WorkbenchData`: a change here renders the frame components that read it (and App,
 * which hands the panels the same `data` and `live` objects), never the panels. `initialArea` overrides the stored area:
 * the app starts on Start, or on Welt when the URL names a project.
 */
export function ShellProvider({ children, initialArea }: { children: ReactNode; initialArea?: AreaId }) {
  const [state, setState] = useState<ShellState>(() => {
    const read = readShellState();
    return initialArea === undefined ? read : { ...read, area: initialArea };
  });
  const [tool, setTool] = useState<Tool>("select");
  const [theme, setTheme] = useState(effectiveTheme);
  const [density, setDensity] = useState(readDensity);

  useEffect(() => writeShellState(state), [state]);
  // An OS switch while the theme is "System" also arrives here.
  useEffect(() => onThemeChange(setTheme), []);

  const setArea = useCallback((area: AreaId) => setState((s) => (s.area === area ? s : { ...s, area })), []);
  const setListOpen = useCallback((area: AreaId, open: boolean) => setState((s) => ({ ...s, listOpen: { ...s.listOpen, [area]: open } })), []);
  const setHandHeight = useCallback((area: AreaId, height: number) => setState((s) => ({ ...s, handHeight: { ...s.handHeight, [area]: clampHand(height) } })), []);
  const toggleHandCollapsed = useCallback((area: AreaId) => setState((s) => ({ ...s, handCollapsed: { ...s.handCollapsed, [area]: !s.handCollapsed[area] } })), []);
  const setDepth = useCallback((depth: Depth) => setState((s) => (s.depth === depth ? s : { ...s, depth })), []);
  const remember = useCallback((project: RecentProject) => setState((s) => ({ ...s, recent: rememberRecent(s.recent, project) })), []);
  const forget = useCallback((path: string) => setState((s) => ({ ...s, recent: s.recent.filter((p) => p.path !== path) })), []);
  const toggleTheme = useCallback(() => applyTheme(effectiveTheme() === "hell" ? "dunkel" : "hell"), []);
  const toggleDensity = useCallback(() => {
    const next: Density = density === "komfort" ? "kompakt" : "komfort";
    applyDensity(next);
    setDensity(next);
  }, [density]);

  const value = useMemo<ShellApi>(
    () => ({ state, setArea, setListOpen, setHandHeight, toggleHandCollapsed, setDepth, tool, setTool, remember, forget, theme, toggleTheme, density, toggleDensity }),
    [state, setArea, setListOpen, setHandHeight, toggleHandCollapsed, setDepth, tool, remember, forget, theme, toggleTheme, density, toggleDensity],
  );
  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>;
}

export function useShell(): ShellApi {
  const shell = useContext(ShellContext);
  if (!shell) throw new Error("useShell outside of a ShellProvider");
  return shell;
}
