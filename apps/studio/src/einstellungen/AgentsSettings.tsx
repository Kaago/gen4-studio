import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentRemoveGrant, agentSetEnabled, agentSetExpert, agentSetGrant, agentSettings } from "../api";
import type { AgentSettingsDto, AgentStatus, GrantLevel } from "../api";
import { Button, IconButton, Notice, Segmented, Switch } from "../ui/primitives";
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

/** The catalog key of each grant level's word. */
const LEVEL_KEY = { none: "agents.levelNone", read: "agents.levelRead", write: "agents.levelWrite" } as const satisfies Record<GrantLevel, string>;

/** `unset`: the open project has no grant yet, no option is pressed (the app asks on the first access). */
type LevelChoice = GrantLevel | "unset";
const LEVEL_ORDER = ["none", "read", "write"] as const;

/**
 * Einstellungen › Agenten: whether local agents (MCP clients such as Claude Code) may work in the running app, where they reach it,
 * the line that sets up Claude Code, the grant of the open project, the grants of other projects and the expert tools. Reads
 * `agent_settings` when shown, after each change and when the hub's switch or port changes; failures go to `onError`.
 */
export function AgentsSettings({ projectOpen, bridge, status, onError }: AgentsSettingsProps) {
  const { t } = useTranslation("app");
  const levels: readonly SegmentedOption<LevelChoice>[] = LEVEL_ORDER.map((value) => ({ value, label: t(LEVEL_KEY[value]) }));
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
      onError(new Error(t("agents.noClipboard")));
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
  const ready = bridge ? enabled : port !== null;
  const server = !ready ? t("agents.serverOff") : bridge ? t("agents.serverBridge") : t("agents.serverReady", { port });
  const open = projectOpen ? (dto?.open_project ?? null) : null;
  const others = dto?.settings.grants.filter((grant) => grant.path !== open?.path) ?? [];

  return (
    <section className={styles.column} aria-labelledby="settings-agents" data-agents-settings="">
      <h2 id="settings-agents" className={styles.heading}>
        {t("agents.title")}
      </h2>
      {dto !== null && (
        <>
          <div className={styles.field}>
            <div data-agents-enabled="">
              <Switch label={t("agents.allow")} checked={enabled} onChange={(on) => change(() => agentSetEnabled(on))} />
            </div>
            <p className={styles.server}>
              <span className={ready ? `${styles.dot} ${styles.ready}` : styles.dot} aria-hidden="true" />
              <span data-agents-server="">{server}</span>
            </p>
            {status?.server_error && (
              <div className={styles.serverError} data-agents-server-error="">
                <Notice kind="error" title={t("agents.serverError")}>
                  {status.server_error}
                </Notice>
              </div>
            )}
          </div>
          <div className={styles.field}>
            <span className={styles.label}>{t("agents.setup")}</span>
            <code className={styles.setup} data-agents-setup="">
              {dto.setup_line}
            </code>
            <Button kind="secondary" size="s" icon={copied ? "check" : "copy"} onClick={() => copy(dto.setup_line)}>
              {copied ? t("agents.copied") : t("agents.copy")}
            </Button>
          </div>
          <div className={styles.field}>
            <span className={styles.label}>{t("agents.thisProject")}</span>
            {open === null ? (
              <p className={styles.hint}>{t("agents.noProject")}</p>
            ) : (
              <>
                <Segmented options={levels} value={open.level ?? "unset"} onChange={(level) => level !== "unset" && change(() => agentSetGrant(level))} ariaLabel={t("agents.grantLabel")} wrap />
                {open.level === null && <p className={styles.hint}>{t("agents.notSet")}</p>}
              </>
            )}
          </div>
          {others.length > 0 && (
            <div className={styles.field}>
              <span className={styles.label}>{t("agents.otherProjects")}</span>
              <ul className={styles.grants} data-agents-grants="">
                {others.map((grant) => (
                  <li key={`${grant.path}|${grant.base_id ?? ""}`} className={styles.grant}>
                    <span className={styles.path} title={grant.path}>
                      {grant.path}
                    </span>
                    <span className={styles.level}>{t(LEVEL_KEY[grant.level])}</span>
                    <IconButton label={t("agents.removeGrant")} icon="close" onClick={() => change(() => agentRemoveGrant(grant.path))} />
                  </li>
                ))}
              </ul>
            </div>
          )}
          <div className={styles.field}>
            <div data-agents-expert="">
              <Switch label={t("agents.expert")} checked={dto.settings.expert} onChange={(on) => change(() => agentSetExpert(on))} />
            </div>
            <p className={styles.hint}>{t("agents.expertHint")}</p>
          </div>
        </>
      )}
    </section>
  );
}
