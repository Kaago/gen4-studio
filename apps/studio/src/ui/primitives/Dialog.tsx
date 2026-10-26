import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import type { ReactNode } from "react";
import { Button } from "./Button";
import type { ButtonProps } from "./Button";
import { Icon } from "./Icon";
import styles from "./Dialog.module.css";

/**
 * A modal dialog that is open: Base UI keeps a closing popup mounted (still `aria-modal`) for its exit transition and marks it
 * `data-closed`; such a popup no longer covers the workbench, so a key right after Escape is the workbench's again.
 */
export const OPEN_MODAL = '[role="dialog"][aria-modal="true"]:not([data-closed])';

export type DialogAction = { label: string; kind?: ButtonProps["kind"]; onClick: () => void; disabled?: boolean };

export type DialogProps = {
  open: boolean;
  /** Called with `false` for Escape, the close button and a click outside; the owner decides. */
  onOpenChange: (open: boolean) => void;
  /** A question when the dialog stands before something irreversible. */
  title: string;
  description?: string;
  /** Buttons at the foot, in order; the last one is the one the eye expects to be primary. */
  actions?: readonly DialogAction[];
  children?: ReactNode;
  /**
   * `palette` anchors the dialog near the top, widens it, keeps the title for screen readers only and drops the close
   * button (Escape closes). Default `dialog`.
   */
  variant?: "dialog" | "palette";
  /** Data attributes for the dialog element itself (the popup with `role="dialog"`), e.g. `{ "data-agent-question": "" }`. */
  data?: Readonly<Record<`data-${string}`, string>>;
};

/** A modal dialog: focus is trapped inside, Escape closes (Base UI). Wizards are not dialogs. */
export function Dialog({ open, onOpenChange, title, description, actions, children, variant = "dialog", data }: DialogProps) {
  const palette = variant === "palette";
  return (
    <BaseDialog.Root
      open={open}
      onOpenChange={(next, details) => {
        // A field inside that used Escape to take back typed text has called `preventDefault`: the dialog stays open.
        if (!next && details.reason === "escape-key" && details.event.defaultPrevented) {
          details.cancel();
          return;
        }
        onOpenChange(next);
      }}
    >
      <BaseDialog.Portal>
        <BaseDialog.Backdrop className={styles.backdrop} />
        <BaseDialog.Popup {...data} className={palette ? `${styles.popup} ${styles.palette}` : styles.popup} aria-modal="true">
          <BaseDialog.Title className={palette ? styles.titleHidden : styles.title}>{title}</BaseDialog.Title>
          {description && <BaseDialog.Description className={styles.description}>{description}</BaseDialog.Description>}
          {children && <div className={styles.body}>{children}</div>}
          {actions && actions.length > 0 && (
            <div className={styles.actions}>
              {actions.map((a) => (
                <Button key={a.label} kind={a.kind ?? "secondary"} onClick={a.onClick} disabled={a.disabled}>
                  {a.label}
                </Button>
              ))}
            </div>
          )}
          {/* After the content in the DOM so the first Tab stop is content, not the cross; placed top right by CSS. */}
          {!palette && (
            <BaseDialog.Close className={styles.close} aria-label="Schließen">
              <Icon name="close" />
            </BaseDialog.Close>
          )}
        </BaseDialog.Popup>
      </BaseDialog.Portal>
    </BaseDialog.Root>
  );
}
