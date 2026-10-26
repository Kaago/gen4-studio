import { useCallback, useEffect, useRef, useState } from "react";
import { agentRemoveGrant, agentSetEnabled, agentSetExpert, agentSetGrant, agentSettings } from "../api";
import type { AgentSettingsDto, AgentStatus, GrantLevel } from "../api";
import { Button, IconButton, Segmented, Switch } from "../ui/primitives";
import type { SegmentedOption } from "../ui/primitives";
import styles from "./AgentsSettings.module.css";

export type AgentsSettingsProps = {
  /** A project is open: the grant segment is for it. A change reads the settings again. */
  projectOpen: boolean;
  /** The browser mode: the dev bridge serves `/mcp` on its own port, there is no server of the app's own. */
  bridge: boolean;
  /**
   * The hub's status as last read. Every new status reads the settings again: an answered question saved a grant, the switch or the
   * port changed (`useAgents` keeps the object while nothing was read).
   */
  status: AgentStatus | null;
  onError: (error: unknown) => void;
};

/** How long the copy button says „Kopiert“. */
export const COPIED_MS = 2000;

const LEVEL_LABEL: Record<GrantLevel, string> = { none: "Nicht", read: "Lesen", write: "Lesen und bearbeiten" };

/** `unset`: the open project has no grant yet, no option is pressed (the app asks on the first access). */
type LevelChoice = GrantLevel | "unset";
const LEVELS: readonly SegmentedOption<LevelChoice>[] = (["none", "read", "write"] as const).map((value) => ({ value, label: LEVEL_LABEL[value] }));

/**
 * Einstellungen › Agenten: whether local agents (MCP clients such as Claude Code) may work in the running app, where they reach it,
 * the line that sets up Claude Code, the grant of the open project, the grants of other projects and the expert tools. Reads
 * `agent_settings` when shown, after each change and when the hub's switch or port changes; failures go to `onError`.
 */
export function AgentsSettings({ projectOpen, bridge, status, onError }: AgentsSettingsProps) {
  const [dto, setDto] = useState<AgentSettingsDto | null>(null);
  const [copied, setCopied] = useState(false);
  // Only the newest read is shown: an older answer that arrives late would show settings from before a change.
  const asked = useRef(0);
  const copiedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const reload = useCallback(async () => {
    const seq = ++asked.current;
    const next = await agentSettings();
    if (seq === asked.current) setDto(next);
  }, []);

  useEffect(() => {
    reload().catch(onError);
    // `onError` is a sink, not a reason to read again.
  }, [reload, projectOpen, status]);

  useEffect(
    () => () => {
      // Answers after unmounting are dropped.
      asked.current++;
      if (copiedTimer.current !== null) clearTimeout(copiedTimer.current);
    },
    [],
  );

  const change = (action: () => Promise<void>) => {
    void action()
      .then(reload)
      .catch((error: unknown) => {
        onError(error);
        reload().catch(onError);
      });
  };

  const copy = (line: string) => {
    // Missing outside a secure context (a browser on another host than localhost).
    const clipboard = navigator.clipboard as Clipboard | undefined;
    if (clipboard === undefined) {
      onError(new Error("Die Zwischenablage ist hier nicht verfügbar. Markiere die Einrichtungszeile und kopiere sie selbst."));
      return;
    }
    void clipboard
      .writeText(line)
      .then(() => {
        setCopied(true);
        if (copiedTimer.current !== null) clearTimeout(copiedTimer.current);
        copiedTimer.current = setTimeout(() => {
          copiedTimer.current = null;
          setCopied(false);
        }, COPIED_MS);
      })
      .catch(onError);
  };

  const enabled = dto?.settings.enabled ?? false;
  const port = status?.port ?? null;
  const server = bridge ? (enabled ? "Über die Brücke (/mcp)" : "Aus") : port !== null ? `Bereit auf 127.0.0.1:${port}` : "Aus";
  const ready = server !== "Aus";
  const open = projectOpen ? (dto?.open_project ?? null) : null;
  const others = dto?.settings.grants.filter((grant) => grant.path !== open?.path) ?? [];

  return (
    <section className={styles.column} aria-labelledby="settings-agents" data-agents-settings="">
      <h2 id="settings-agents" className={styles.heading}>
        Agenten
      </h2>
      {dto !== null && (
        <>
          <div className={styles.field}>
            <div data-agents-enabled="">
              <Switch label="Lokale Agenten zulassen" checked={enabled} onChange={(on) => change(() => agentSetEnabled(on))} />
            </div>
            <p className={styles.server}>
              <span className={ready ? `${styles.dot} ${styles.ready}` : styles.dot} aria-hidden="true" />
              <span data-agents-server="">{server}</span>
            </p>
          </div>
          <div className={styles.field}>
            <span className={styles.label}>Einrichtung in Claude Code</span>
            <code className={styles.setup} data-agents-setup="">
              {dto.setup_line}
            </code>
            <Button kind="secondary" size="s" icon={copied ? "check" : "copy"} onClick={() => copy(dto.setup_line)}>
              {copied ? "Kopiert" : "Einrichtungszeile kopieren"}
            </Button>
          </div>
          <div className={styles.field}>
            <span className={styles.label}>Dieses Projekt</span>
            {open === null ? (
              <p className={styles.hint}>Kein Projekt offen</p>
            ) : (
              <>
                <Segmented options={LEVELS} value={open.level ?? "unset"} onChange={(level) => level !== "unset" && change(() => agentSetGrant(level))} ariaLabel="Freigabe für dieses Projekt" wrap />
                {open.level === null && <p className={styles.hint}>Noch nicht festgelegt: Gen4 Studio fragt beim ersten Zugriff eines Agenten</p>}
              </>
            )}
          </div>
          {others.length > 0 && (
            <div className={styles.field}>
              <span className={styles.label}>Andere Projekte</span>
              <ul className={styles.grants} data-agents-grants="">
                {others.map((grant) => (
                  <li key={`${grant.path}|${grant.base_id ?? ""}`} className={styles.grant}>
                    <span className={styles.path} title={grant.path}>
                      {grant.path}
                    </span>
                    <span className={styles.level}>{LEVEL_LABEL[grant.level]}</span>
                    <IconButton label="Freigabe entfernen" icon="close" onClick={() => change(() => agentRemoveGrant(grant.path))} />
                  </li>
                ))}
              </ul>
            </div>
          )}
          <div className={styles.field}>
            <div data-agents-expert="">
              <Switch label="Expertenwerkzeuge (g4_call)" checked={dto.settings.expert} onChange={(on) => change(() => agentSetExpert(on))} />
            </div>
            <p className={styles.hint}>Gibt Agenten jeden Befehl des Kernels, auch ungeprüfte</p>
          </div>
        </>
      )}
    </section>
  );
}
