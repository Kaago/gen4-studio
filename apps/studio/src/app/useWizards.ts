import { useCallback, useEffect, useRef, useState } from "react";
import type { WizardRequest } from "../workbench/WorkbenchContext";
import { useStableCallback } from "./useStable";

/** What a wizard made that the app is to show: the matrix and the header that now has it (null: none was assigned), and the message to leave behind. */
export interface Created {
  matrix: number;
  header: number | null;
  announcement: string;
  /** The name the wizard asked for: the world tree calls the header so (`namedByWizard`). */
  name?: string;
}

interface WizardsParams {
  /**
   * The project changed under the wizard: read it again and show `created`. Called when a wizard
   * was left after it wrote (with null: only read), and by `created`.
   */
  onCreated: (created: Created | null) => Promise<void>;
  /** A wizard opens: the app shows the world (its step bar is in the world's hinge) and puts the tool back on "Auswählen". */
  onOpen?: () => void;
  /**
   * A wizard was left without showing what it made ("Abbrechen", Escape, "Schließen"): the app goes back to the area it was opened from,
   * in the same render as the wizard goes. The focus then goes back to what had it (`returnFocus`).
   */
  onLeave?: () => void;
}

/** What can hold the focus when the wizard was left. */
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

const shown = (el: HTMLElement) => el.isConnected && el.closest("[hidden], [inert]") === null;
/** The name a control is known by (its label, else its text); empty for one without. */
const nameOf = (el: HTMLElement) => (el.getAttribute("aria-label") ?? el.textContent ?? "").trim();

/**
 * Where the focus goes after a wizard was left, unless its frame gave it back already: to what had it when the wizard opened, if that
 * still stands; else to the control of the same kind and name in the area shown now (the area was rebuilt: the button of Technik that
 * opened it is a new one); else to that area's first control. What opened it from the palette is the palette's field, which is gone.
 */
export function returnFocus(opener: HTMLElement | null) {
  const active = document.activeElement;
  if (active !== null && active !== document.body) return;
  if (opener !== null && shown(opener)) {
    opener.focus();
    if (document.activeElement === opener) return;
  }
  const area = document.querySelector<HTMLElement>("main:not([hidden])");
  if (area === null) return;
  const controls = [...area.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(shown);
  const name = opener === null ? "" : nameOf(opener);
  const twin = name === "" ? undefined : controls.find((el) => el.tagName === opener!.tagName && nameOf(el) === name);
  (twin ?? controls[0])?.focus();
}

/** Which wizard is open (if any) with what it is prefilled with. Only the app's wizard host reads `request`. */
export function useWizards({ onCreated, onOpen, onLeave }: WizardsParams) {
  const [request, setRequest] = useState<WizardRequest | null>(null);
  const refresh = useStableCallback(onCreated);
  const opening = useStableCallback(() => onOpen?.());
  const leaving = useStableCallback(() => onLeave?.());
  // What had the focus when the open wizard opened (undefined: none is open).
  const opener = useRef<HTMLElement | null | undefined>(undefined);
  // Left without "Zeigen": where the focus goes once the wizard is gone (undefined: nowhere).
  const back = useRef<HTMLElement | null | undefined>(undefined);

  /** Stable: it is handed to memoised panels as `onOpenWizard`. */
  const open = useCallback(
    (next: WizardRequest) => {
      if (opener.current === undefined) {
        const active = document.activeElement;
        opener.current = active instanceof HTMLElement && active !== document.body ? active : null;
      }
      opening();
      setRequest(next);
    },
    [opening],
  );
  /** Leaves the wizard; `written` says it changed the project, which is then read again (nothing is shown). */
  const close = useCallback(
    (written = false) => {
      setRequest(null);
      if (opener.current !== undefined) {
        back.current = opener.current;
        opener.current = undefined;
        leaving();
      }
      if (written) void refresh(null);
    },
    [refresh, leaving],
  );
  /** "Zeigen": leaves the wizard, reads the project again and shows what it made. */
  const created = useCallback(
    (made: Created) => {
      opener.current = undefined;
      setRequest(null);
      return refresh(made);
    },
    [refresh],
  );

  // After the render that removed the wizard (its frame has given the focus back if it could) and showed the area it came from.
  useEffect(() => {
    if (request !== null || back.current === undefined) return;
    const to = back.current;
    back.current = undefined;
    returnFocus(to);
  }, [request]);

  /** Stable: reads the project again and shows `made` as after a wizard, without touching an open one (a room split off in the inspector). */
  const reread = useCallback((made: Created) => refresh(made), [refresh]);

  return { request, open, close, created, reread };
}

export type Wizards = ReturnType<typeof useWizards>;
