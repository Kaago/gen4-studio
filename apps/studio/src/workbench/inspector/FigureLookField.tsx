import { useEffect, useMemo, useRef, useState } from "react";
import { Button, InspectorRow, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { peekSprite, spriteTexture } from "../../world/sprites";
import type { SpriteTexture } from "../../world/sprites";
import { useVisible } from "../useVisible";
import styles from "./Editor.module.css";

/**
 * How many looks the picker offers (graphics ids 0 to 1023). The frontend cannot list the game's overworld table, so the picker offers
 * every id up to this bound; one without a sprite shows a flat tile.
 */
const FIGURE_LOOKS = 1024;

/** The pictures drawn from the sprite cache's textures, once per texture (null: no canvas to draw on). */
const pictures = new WeakMap<SpriteTexture, string | null>();

/** A sprite texture as a PNG data URL; its rows are stored bottom first for the GPU, so they are turned upright again. */
function spritePicture(sprite: SpriteTexture): string | null {
  const known = pictures.get(sprite);
  if (known !== undefined) return known;
  const { width, height } = sprite;
  const data = (sprite.texture.image as { data: Uint8Array }).data;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  let url: string | null = null;
  if (context) {
    const image = context.createImageData(width, height);
    const row = width * 4;
    for (let y = 0; y < height; y++) image.data.set(data.subarray((height - 1 - y) * row, (height - y) * row), y * row);
    context.putImageData(image, 0, 0);
    url = canvas.toDataURL("image/png");
  }
  pictures.set(sprite, url);
  return url;
}

/** The overworld sprite of `graphicsId` facing `dir`, read only while in view; a flat tile until (or unless) it comes. */
function FigurePreview({ graphicsId, dir }: { graphicsId: number; dir: number }) {
  const tile = useRef<HTMLSpanElement>(null);
  const visible = useVisible(tile, true);
  const [picture, setPicture] = useState<string | null>(() => {
    const known = peekSprite(graphicsId, dir);
    return known ? spritePicture(known) : null;
  });
  useEffect(() => {
    if (!visible || peekSprite(graphicsId, dir) !== undefined) return;
    let cancelled = false;
    void spriteTexture(graphicsId, dir).then((sprite) => {
      if (!cancelled) setPicture(sprite ? spritePicture(sprite) : null);
    });
    return () => {
      cancelled = true;
    };
  }, [graphicsId, dir, visible]);
  return <span ref={tile} className={styles.figure} style={picture !== null ? { backgroundImage: `url("${picture}")` } : undefined} /* dyn */ data-figure-look={graphicsId} />;
}

/**
 * A figure's field „Aussehen“ in Erzählen: its overworld sprite in the facing it has, and „Ändern…“, a picker of the looks with their
 * sprites; a pick is one undoable change of `graphics_id`.
 */
export function FigureLookField({ value, dir, disabled, onCommit }: { value: number; dir: number; disabled?: boolean; onCommit: (graphicsId: number) => void }) {
  const [picking, setPicking] = useState(false);
  const items = useMemo<PickerItem[]>(
    () =>
      picking
        ? Array.from({ length: Math.max(FIGURE_LOOKS, value + 1) }, (_, n) => ({ id: n, label: `Aussehen ${n}`, area: "welt" as const, preview: <FigurePreview graphicsId={n} dir={dir} /> }))
        : [],
    [picking, value, dir],
  );
  return (
    <>
      <InspectorRow label="Aussehen">
        <div className={styles.inline}>
          <FigurePreview key={`${value}:${dir}`} graphicsId={value} dir={dir} />
          <Button kind="quiet" size="s" aria-label="Aussehen ändern…" disabled={disabled} onClick={() => setPicking(true)}>
            Ändern…
          </Button>
        </div>
      </InspectorRow>
      <Picker open={picking} onOpenChange={setPicking} title="Aussehen wählen" items={items} placeholder="Nummer suchen" onPick={(item) => onCommit(Number(item.id))} />
    </>
  );
}
