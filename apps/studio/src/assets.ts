import type { AssetId, AssetKind } from "./api";
import { t } from "./i18n";

/** Display name of an asset kind in the language shown (`common:assets.kinds`, its key the kind with a small first letter). */
export function kindLabel(kind: AssetKind): string {
  const key = `${kind.charAt(0).toLowerCase()}${kind.slice(1)}` as Uncapitalize<AssetKind>;
  return t(`common:assets.kinds.${key}`);
}

/** "Karte 0000": the kind's name and four-digit member index. */
export function assetLabel(asset: AssetId): string {
  return `${kindLabel(asset.kind)} ${String(asset.index).padStart(4, "0")}`;
}

/** Message of the bar when an asset changed on disk behind the editor's back. */
export function staleText(asset: AssetId): string {
  return t("common:assets.stale", { asset: assetLabel(asset) });
}
