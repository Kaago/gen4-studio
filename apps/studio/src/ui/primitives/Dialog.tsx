import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./Button";
import type { ButtonProps } from "./Button";
import { Icon } from "./Icon";
import styles from "./Dialog.module.css";

/**
 * A modal dialog that is open: Base UI keeps a closing popup mounted (still `aria-modal`) for its exit transition and marks it
 * `data-closed`; such a popup no longer covers the workbench, so a key right after Escape is the workbench's again.
 */
export const OPEN_MODAL = '[role="dialog"][aria-modal="true"]:not([data-closed])';

export type DialogAction = {
  label: string;
  kind?: ButtonProps["kind"];
  onClick: () => void;
  disabled?: boolean;
  /** Data attributes of the button (a marker a test or the gate finds it by). */
  data?: Readonly<Record<`data-${string}`, string>>;
};

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
  /**
   * Where focus goes once the dialog has closed: asked then, not when it opens (the opener may be gone or disabled by that time);
   * `null` leaves it to Base UI, which returns focus to the element that had it when the dialog opened.
   */
  finalFocus?: () => HTMLElement | null;
  /**
   * False while the dialog must stay (work that cannot be left runs in it): no close button, and Escape and a click outside do nothing.
   * Default true.
   */
  dismissible?: boolean;
  /** `wide` widens a dialog to 960 px for work in two columns (the dialog editor of a figure). Default `standard` (480 px). */
  size?: "standard" | "wide";
};

/** A modal dialog: focus is trapped inside, Escape closes (Base UI). Wizards are not dialogs. */
export function Dialog({ open, onOpenChange, title, description, actions, children, variant = "dialog", data, finalFocus, dismissible = true, size = "standard" }: DialogProps) {
  const { t } = useTranslation("ui");
  const palette = variant === "palette";
  const popupClass = palette ? `${styles.popup} ${styles.palette}` : size === "wide" ? `${styles.popup} ${styles.wide}` : styles.popup;
  return (
    <BaseDialog.Root
      open={open}
      onOpenChange={(next, details) => {
        if (!next && !dismissible) {
          details.cancel();
          return;
        }
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
        <BaseDialog.Popup {...data} finalFocus={finalFocus} className={popupClass} aria-modal="true">
          <BaseDialog.Title className={palette ? styles.titleHidden : styles.title}>{title}</BaseDialog.Title>
          {description && <BaseDialog.Description className={styles.description}>{description}</BaseDialog.Description>}
          {children && <div className={styles.body}>{children}</div>}
          {actions && actions.length > 0 && (
            <div className={styles.actions}>
              {actions.map((a) => (
                <Button key={a.label} {...a.data} kind={a.kind ?? "secondary"} onClick={a.onClick} disabled={a.disabled}>
                  {a.label}
                </Button>
              ))}
            </div>
          )}
          {/* After the content in the DOM so the first Tab stop is content, not the cross; placed top right by CSS. */}
          {!palette && dismissible && (
            <BaseDialog.Close className={styles.close} aria-label={t("dialog.close")}>
              <Icon name="close" />
            </BaseDialog.Close>
          )}
        </BaseDialog.Popup>
      </BaseDialog.Portal>
    </BaseDialog.Root>
  );
}
