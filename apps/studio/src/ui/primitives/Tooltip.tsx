import { Tooltip as BaseTooltip } from "@base-ui/react/tooltip";
import type { ReactElement, ReactNode } from "react";
import styles from "./Tooltip.module.css";

export type TooltipProps = {
  content: ReactNode;
  /** The element that gets the tooltip; it becomes the trigger (Base UI `render`). */
  children: ReactElement<Record<string, unknown>>;
  /** Shows nothing while true; the trigger stays the same element (a control that sometimes needs a tooltip keeps its focus). */
  disabled?: boolean;
};

/** Opens after 400 ms of hover or on keyboard focus. */
export function Tooltip({ content, children, disabled = false }: TooltipProps) {
  return (
    <BaseTooltip.Root disabled={disabled}>
      <BaseTooltip.Trigger delay={400} render={children} />
      <BaseTooltip.Portal>
        <BaseTooltip.Positioner sideOffset={6} className={styles.positioner}>
          <BaseTooltip.Popup role="tooltip" className={styles.popup}>{content}</BaseTooltip.Popup>
        </BaseTooltip.Positioner>
      </BaseTooltip.Portal>
    </BaseTooltip.Root>
  );
}
