import { icons } from "./icons";
import type { IconName } from "./icons";

export type IconProps = {
  name: IconName;
  weight?: "regular" | "fill";
};

/** Decorative symbol sized by the surrounding `font-size` (containers set `--g4-dichte-symbol`). */
export function Icon({ name, weight = "regular" }: IconProps) {
  const Glyph = icons[name];
  return <Glyph size="1em" weight={weight} aria-hidden="true" focusable="false" />;
}
