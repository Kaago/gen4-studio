import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { applyDensity, applyTheme, effectiveTheme, onThemeChange, readDensity, readTheme } from "../tokens/theme";
import type { Density, Theme } from "../tokens/theme";
import { clampHand, readShellState, rememberRecent, writeShellState } from "./shellState";
import type { AreaId, DataSelectKey, Depth, PokemonSub, RecentProject, ShellState, Tool } from "./shellState";

export interface ShellApi {
  state: ShellState;
  setArea: (area: AreaId) => void;
  setListOpen: (area: AreaId, open: boolean) => void;
  setHandHeight: (area: AreaId, height: number) => void;
  toggleHandCollapsed: (area: AreaId) => void;
  setDepth: (depth: Depth) => void;
  /** The part the Pokémon area shows (whether it may is the area's to decide). */
  setPokemonSub: (sub: PokemonSub) => void;
  /** The selection of a data part; null: none. */
  setSelected: (key: DataSelectKey, index: number | null) => void;
  /** The tool of the scene; not remembered between runs. Any tool but "figure" drops `pendingTrainer`. */
  tool: Tool;
  setTool: (tool: Tool) => void;
  /** The trainer the next figure is placed as („In der Welt platzieren“); null: a plain figure. Transient like the tool, never stored. */
  pendingTrainer: number | null;
  setPendingTrainer: (id: number | null) => void;
  /** Puts a project first in the recent list (and drops an earlier entry of the same path). */
  remember: (project: RecentProject) => void;
  forget: (path: string) => void;
  /** The theme that is shown; the title bar button switches hell and dunkel ("System" is a setting). */
  theme: "hell" | "dunkel";
  /** The theme as chosen in the settings: "system" follows the OS. */
  themeSetting: Theme;
  /** Applies and remembers a theme setting (the settings' segmented control). */
  setTheme: (t: Theme) => void;
  /** Switches the shown theme and fixes the setting to it. */
  toggleTheme: () => void;
  density: Density;
  setDensity: (d: Density) => void;
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
  const [tool, setToolState] = useState<Tool>("select");
  const [pendingTrainer, setPendingTrainer] = useState<number | null>(null);
  // Escape, another tool, or the figure's click going back to "Auswählen" all end the binding of a trainer to the figure tool.
  const setTool = useCallback((next: Tool) => {
    setToolState(next);
    if (next !== "figure") setPendingTrainer(null);
  }, []);
  const [theme, setShownTheme] = useState(effectiveTheme);
  const [themeSetting, setThemeSetting] = useState(readTheme);
  const [density, setDensityState] = useState(readDensity);

  useEffect(() => writeShellState(state), [state]);
  // An OS switch while the theme is "System" also arrives here.
  useEffect(() => onThemeChange(setShownTheme), []);

  const setArea = useCallback((area: AreaId) => setState((s) => (s.area === area ? s : { ...s, area })), []);
  const setListOpen = useCallback((area: AreaId, open: boolean) => setState((s) => ({ ...s, listOpen: { ...s.listOpen, [area]: open } })), []);
  const setHandHeight = useCallback((area: AreaId, height: number) => setState((s) => ({ ...s, handHeight: { ...s.handHeight, [area]: clampHand(height) } })), []);
  const toggleHandCollapsed = useCallback((area: AreaId) => setState((s) => ({ ...s, handCollapsed: { ...s.handCollapsed, [area]: !s.handCollapsed[area] } })), []);
  const setDepth = useCallback((depth: Depth) => setState((s) => (s.depth === depth ? s : { ...s, depth })), []);
  const setPokemonSub = useCallback((sub: PokemonSub) => setState((s) => (s.sub.pokemon === sub ? s : { ...s, sub: { ...s.sub, pokemon: sub } })), []);
  const setSelected = useCallback(
    (key: DataSelectKey, index: number | null) =>
      setState((s) => {
        if ((s.selected[key] ?? null) === index) return s;
        const selected = { ...s.selected };
        if (index === null) delete selected[key];
        else selected[key] = index;
        return { ...s, selected };
      }),
    [],
  );
  const remember = useCallback((project: RecentProject) => setState((s) => ({ ...s, recent: rememberRecent(s.recent, project) })), []);
  const forget = useCallback((path: string) => setState((s) => ({ ...s, recent: s.recent.filter((p) => p.path !== path) })), []);
  const setTheme = useCallback((t: Theme) => {
    applyTheme(t);
    setThemeSetting(t);
    // The listener above hears only an effective switch; set it here too, so the shown theme is right whatever the listener did.
    setShownTheme(effectiveTheme());
  }, []);
  const toggleTheme = useCallback(() => setTheme(effectiveTheme() === "hell" ? "dunkel" : "hell"), [setTheme]);
  const setDensity = useCallback((d: Density) => {
    applyDensity(d);
    setDensityState(d);
  }, []);
  const toggleDensity = useCallback(() => setDensity(density === "komfort" ? "kompakt" : "komfort"), [density, setDensity]);

  const value = useMemo<ShellApi>(
    () => ({
      state,
      setArea,
      setListOpen,
      setHandHeight,
      toggleHandCollapsed,
      setDepth,
      setPokemonSub,
      setSelected,
      tool,
      setTool,
      pendingTrainer,
      setPendingTrainer,
      remember,
      forget,
      theme,
      themeSetting,
      setTheme,
      toggleTheme,
      density,
      setDensity,
      toggleDensity,
    }),
    [state, setArea, setListOpen, setHandHeight, toggleHandCollapsed, setDepth, setPokemonSub, setSelected, tool, setTool, pendingTrainer, remember, forget, theme, themeSetting, setTheme, toggleTheme, density, setDensity, toggleDensity],
  );
  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>;
}

export function useShell(): ShellApi {
  const shell = useContext(ShellContext);
  if (!shell) throw new Error("useShell outside of a ShellProvider");
  return shell;
}
