import { useCallback, useRef } from "react";

/**
 * A function with a fixed identity that always calls the latest `fn`. For callbacks that are
 * plain closures over the current render (they read state through refs or setters anyway) but are
 * handed to memoised panels: a new closure every render would defeat the memo.
 */
export function useStableCallback<A extends unknown[], R>(fn: (...args: A) => R): (...args: A) => R {
  const latest = useRef(fn);
  latest.current = fn;
  return useCallback((...args: A) => latest.current(...args), []);
}

export function shallowEqual(a: object, b: object): boolean {
  if (a === b) return true;
  const keys = Object.keys(a);
  if (keys.length !== Object.keys(b).length) return false;
  const left = a as Record<string, unknown>;
  const right = b as Record<string, unknown>;
  return keys.every((key) => Object.prototype.hasOwnProperty.call(right, key) && Object.is(left[key], right[key]));
}

/**
 * The previous object while `next` has the same fields (compared with `Object.is`), else `next`.
 * The identity changes exactly when a field does, with no dependency list to keep in step with
 * the fields: use it for a context value built from many pieces.
 */
export function useShallowStable<T extends object>(next: T): T {
  const previous = useRef(next);
  if (!shallowEqual(previous.current, next)) previous.current = next;
  return previous.current;
}
