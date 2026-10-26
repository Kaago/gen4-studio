import { DSScreen } from "../primitives";
import styles from "./ProjectPreview.module.css";

/** The DS screen is 256 × 192 pixels; the preview picture is exactly that size. */
const WIDTH = 256;
const HEIGHT = 192;

export type ProjectPreviewProps = {
  /** The project's name: shown on the empty screen while there is no picture. */
  name: string;
  /** Base64 of the PNG (256 × 192), or null for none (not loaded yet, or the project has none). */
  png: string | null;
};

/**
 * The picture of a project on a DS screen (phase 4.0 design, §3.5), or its name on the empty screen when there is none. Both are only
 * the picture: the picture has `alt=""` and the name is hidden from the accessibility tree, since whoever shows this names the project
 * beside it. The screen magnifies by whole numbers, so the pixels stay sharp.
 */
export function ProjectPreview({ name, png }: ProjectPreviewProps) {
  return (
    <DSScreen
      variant="empty"
      pages={[]}
      page={0}
      background={
        png !== null ? (
          <img className={styles.picture} src={`data:image/png;base64,${png}`} alt="" width={WIDTH} height={HEIGHT} draggable={false} />
        ) : (
          <span className={styles.name} data-preview-name="" aria-hidden="true">
            {name}
          </span>
        )
      }
    />
  );
}
