import { Sprite } from "../../ui/primitives";
import { trainerSprite, trainerSpriteKey, useIcon } from "../icons";

/**
 * The picture of a trainer class (80 × 80, drawn pixel sharp at `size`) beside the head of a trainer's view or hand. It only decorates:
 * the title next to it names the trainer. `cls` null (a placeholder, a locked trainer: its class is not to be trusted) asks for nothing and
 * shows the placeholder; a class without a picture shows it too.
 */
export function ClassSprite({ cls, name, size }: { cls: number | null; name: string; size: 48 | 80 }) {
  const url = useIcon(cls === null ? null : trainerSpriteKey(cls), () => trainerSprite(cls ?? 0), true);
  return (
    <span data-trainer-sprite="">
      <Sprite url={url} label={name} size={size} area="pokemon" decorative />
    </span>
  );
}
