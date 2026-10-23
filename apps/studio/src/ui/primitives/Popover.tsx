import { Popover as BasePopover } from "@base-ui/react/popover";
import type { ReactNode } from "react";
import menuStyles from "./Menu.module.css";
import styles from "./Popover.module.css";

export type PopoverProps = {
  /** The content of the button that opens the popover. It is not interactive itself. */
  trigger: ReactNode;
  /** The accessible name of the trigger; required when `trigger` is a symbol without text. */
  label?: string;
  children: ReactNode;
};

/** A small non-modal panel next to its trigger: Escape and a click outside close it. */
export function Popover({ trigger, label, children }: PopoverProps) {
  return (
    <BasePopover.Root>
      <BasePopover.Trigger className={menuStyles.trigger} aria-label={label}>
        {trigger}
      </BasePopover.Trigger>
      <BasePopover.Portal>
        <BasePopover.Positioner sideOffset={6} className={styles.positioner}>
          <BasePopover.Popup className={styles.popup}>{children}</BasePopover.Popup>
        </BasePopover.Positioner>
      </BasePopover.Portal>
    </BasePopover.Root>
  );
}
