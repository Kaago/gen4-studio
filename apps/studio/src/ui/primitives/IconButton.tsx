import { Toggle } from "@base-ui/react/toggle";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import { Kbd } from "./Kbd";
import type { RowDataAttrs } from "./ListRow";
import { Tooltip } from "./Tooltip";
import styles from "./IconButton.module.css";

export type IconButtonProps = {
  /** Required: becomes the accessible name and the tooltip. */
  label: string;
  icon: IconName;
  shortcut?: string;
  /** Present for toggle buttons: sets `aria-pressed`. */
  pressed?: boolean;
  disabled?: boolean;
  /** Blocked for a moment (a build runs): looks disabled through `aria-disabled` but keeps keyboard focus; a click does nothing. */
  busy?: boolean;
  /** Why it is blocked, or what else to know: the tooltip's second line and the `aria-description`. */
  hint?: string;
  onClick: () => void;
  /** `data-*` attributes of the button, to find it by something other than its (translated) label. */
  dataAttrs?: RowDataAttrs;
};

export function IconButton({ label, icon, shortcut, pressed, disabled, busy = false, hint, onClick, dataAttrs }: IconButtonProps) {
  const click = () => {
    if (!busy) onClick();
  };
  const name = (
    <>
      {label}
      {shortcut && (
        <>
          {" "}
          <Kbd keys={shortcut} />
        </>
      )}
    </>
  );
  const tip = hint ? (
    <span className={styles.tip}>
      <span data-tip-line="">{name}</span>
      <span className={styles.hint} data-tip-line="">
        {hint}
      </span>
    </span>
  ) : (
    name
  );
  const glyph = <Icon name={icon} weight={pressed ? "fill" : "regular"} />;
  const button =
    pressed === undefined ? (
      <button type="button" className={styles.button} aria-label={label} aria-description={hint} aria-disabled={busy || undefined} disabled={disabled} onClick={click} {...dataAttrs}>
        {glyph}
      </button>
    ) : (
      <Toggle
        className={styles.button}
        aria-label={label}
        aria-description={hint}
        pressed={pressed}
        aria-disabled={busy || undefined}
        disabled={disabled}
        onClick={click}
        {...dataAttrs}
        render={<button type="button" />}
      >
        {glyph}
      </Toggle>
    );
  return <Tooltip content={tip}>{button}</Tooltip>;
}
