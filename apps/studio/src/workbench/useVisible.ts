import { useEffect, useState } from "react";
import type { RefObject } from "react";

/** Whether the element is (nearly) in view; always true where there is no IntersectionObserver (tests). */
export function useVisible(ref: RefObject<HTMLElement | null>, active: boolean): boolean {
  const [visible, setVisible] = useState(() => typeof IntersectionObserver === "undefined");
  useEffect(() => {
    const el = ref.current;
    if (!active || !el || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver((entries) => setVisible(entries[entries.length - 1]?.isIntersecting ?? false), { rootMargin: "128px" });
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref, active]);
  return visible;
}
