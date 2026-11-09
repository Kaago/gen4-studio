import { useTranslation } from "react-i18next";
import type { AgentStatus as AgentStatusDto } from "../../api";
import { t as translate } from "../../i18n";
import { Icon, Menu } from "../primitives";
import styles from "./AgentStatus.module.css";

export type AgentStatusProps = {
  /** The hub's status as last read; without a connection the item is not there. */
  status: AgentStatusDto | null;
  /** „Trennen“: closes every agent session. */
  onDisconnect: () => void;
  /** „Nur lesen“ with the new state of the switch. */
  onReadOnly: (on: boolean) => void;
};

/** What the status line says about connected agents: one by its client's name, more by their count, „Nur lesen“ when it is on. */
export function agentStatusText(status: AgentStatusDto): string {
  const { connections } = status;
  const who = translate("ui:agentStatus.connected", { count: connections.length, client: connections[0]?.client ?? "" });
  return status.read_only ? translate("ui:agentStatus.withReadOnly", { who }) : who;
}

/**
 * The status line's item for local agents (MCP clients working in this session): the agent symbol and who is connected, with a menu
 * for the runtime switch „Nur lesen“ (checked while on) and „Trennen“. Nothing while no agent is connected.
 */
export function AgentStatus({ status, onDisconnect, onReadOnly }: AgentStatusProps) {
  const { t } = useTranslation("ui");
  if (status === null || status.connections.length === 0) return null;
  const readOnly = status.read_only;
  return (
    <span className={styles.item} data-agent-status="" data-agent-read-only={readOnly ? "" : undefined}>
      <Menu
        trigger={
          <>
            <Icon name="agent" />
            {agentStatusText(status)}
          </>
        }
        items={[
          { label: t("agentStatus.readOnly"), checked: readOnly, onSelect: () => onReadOnly(!readOnly) },
          { label: t("agentStatus.disconnect"), onSelect: onDisconnect },
        ]}
      />
    </span>
  );
}
