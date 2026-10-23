import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import type { ReactNode } from "react";
import { Button } from "./Button";
import type { ButtonProps } from "./Button";
import { Icon } from "./Icon";
import styles from "./Dialog.module.css";

export type DialogAction = { label: string; kind?: ButtonProps["kind"]; onClick: () => void };

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
};

/** A modal dialog: focus is trapped inside, Escape closes (Base UI). Wizards are not dialogs. */
export function Dialog({ open, onOpenChange, title, description, actions, children }: DialogProps) {
  return (
    <BaseDialog.Root open={open} onOpenChange={(next) => onOpenChange(next)}>
      <BaseDialog.Portal>
        <BaseDialog.Backdrop className={styles.backdrop} />
        <BaseDialog.Popup className={styles.popup} aria-modal="true">
          <BaseDialog.Title className={styles.title}>{title}</BaseDialog.Title>
          {description && <BaseDialog.Description className={styles.description}>{description}</BaseDialog.Description>}
          {children && <div className={styles.body}>{children}</div>}
          {actions && actions.length > 0 && (
            <div className={styles.actions}>
              {actions.map((a) => (
                <Button key={a.label} kind={a.kind ?? "secondary"} onClick={a.onClick}>
                  {a.label}
                </Button>
              ))}
            </div>
          )}
          {/* After the content in the DOM so the first Tab stop is content, not the cross; placed top right by CSS. */}
          <BaseDialog.Close className={styles.close} aria-label="Schließen">
            <Icon name="close" />
          </BaseDialog.Close>
        </BaseDialog.Popup>
      </BaseDialog.Portal>
    </BaseDialog.Root>
  );
}
