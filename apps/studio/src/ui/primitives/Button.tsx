import type { ButtonHTMLAttributes, Ref } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import { Kbd } from "./Kbd";
import styles from "./Button.module.css";

export type ButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "style"> & {
  kind: "primary" | "secondary" | "quiet" | "play" | "danger";
  size?: "m" | "s";
  icon?: IconName;
  /** Shown as a key hint, e.g. "⌘S". */
  shortcut?: string;
  /** React 19 passes `ref` as a prop: it reaches the <button> through the rest props. */
  ref?: Ref<HTMLButtonElement>;
};

export function Button({ kind, size = "m", icon, shortcut, children, type = "button", ...rest }: ButtonProps) {
  return (
    <button {...rest} type={type} data-kind={kind} className={`${styles.button} ${styles[kind]} ${styles[size]}`}>
      {icon && <Icon name={icon} weight={kind === "play" ? "fill" : "regular"} />}
      {children}
      {shortcut && <Kbd keys={shortcut} />}
    </button>
  );
}
