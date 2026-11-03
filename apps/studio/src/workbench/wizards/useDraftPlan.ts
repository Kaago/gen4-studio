import { useEffect, useRef, useState } from "react";
import { errorText, recipePlan } from "../../api";
import type { DraftDto, RecipeId } from "../../api";
import { useStableCallback } from "../../app/useStable";

/** How long the input must stay unchanged before the plan is asked for. */
export const DRAFT_DEBOUNCE_MS = 300;

type DraftState = {
  draft: DraftDto | null;
  error: string | null;
  /** The draft or the error answers the current input (false while a changed input waits for its answer and the last draft stays). */
  current: boolean;
};

/** The answer held: the draft or error and the input (serialized) it belongs to. */
type Answer = { draft: DraftDto | null; error: string | null; key: string | null };

const NONE: Answer = { draft: null, error: null, key: null };

/**
 * The draft of `recipe` for `input` (null while the input is incomplete, the plan failed or no answer is in yet). Asks 300 ms after
 * the last change; an answer for an older input is dropped; `null` input clears at once. Reports every change through `onDraft`.
 *
 * The input counts by value (a wizard builds its spec anew on every render). While a changed input waits for its answer, the last
 * draft stays, so the scene does not flicker on every typed number. Going away (the wizard closes) reports null.
 */
export function useDraftPlan(recipe: RecipeId, input: unknown | null, onDraft: (draft: DraftDto | null) => void): DraftState {
  const key = input === null ? null : JSON.stringify(input);
  const [state, setState] = useState<Answer>(NONE);
  const report = useStableCallback(onDraft);
  // The draft `onDraft` was last told: a change is reported once.
  const reported = useRef<DraftDto | null>(null);
  const show = useStableCallback((draft: DraftDto | null, error: string | null, of: string | null) => {
    setState((s) => (s.draft === draft && s.error === error && s.key === of ? s : { draft, error, key: of }));
    if (reported.current === draft) return;
    reported.current = draft;
    report(draft);
  });

  useEffect(() => {
    if (key === null) {
      show(null, null, null);
      return;
    }
    // Set false when the input changes again or the wizard goes: that answer is no longer wanted.
    let live = true;
    const timer = setTimeout(() => {
      recipePlan(recipe, JSON.parse(key)).then(
        (draft) => live && show(draft, null, key),
        (e) => live && show(null, errorText(e), key),
      );
    }, DRAFT_DEBOUNCE_MS);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [recipe, key]);

  useEffect(
    () => () => {
      if (reported.current === null) return;
      reported.current = null;
      report(null);
    },
    [],
  );

  return { draft: state.draft, error: state.error, current: state.key === key };
}
