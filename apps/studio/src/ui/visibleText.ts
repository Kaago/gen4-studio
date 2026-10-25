/** Whether an element is left out with everything in it: `hidden`, `aria-hidden="true"` or `display: none`. */
function concealed(el: Element): boolean {
  if (el.hasAttribute("hidden") || el.getAttribute("aria-hidden") === "true") return true;
  const view = el.ownerDocument.defaultView;
  return view !== null && view.getComputedStyle(el).display === "none";
}

/**
 * The words a user sees under `root`: the text of every element that is not `hidden`, not `aria-hidden="true"` and not
 * `display: none` (nor inside one), plus the placeholders of fields; of a native select only the chosen option. Tooltips
 * (`title`), accessible names (`aria-label`) and field values (the project's data) are not visible text. Self-contained (no
 * imports), so the Gate can run it in the app's page as it is.
 */
export function visibleWords(root: Element): string {
  const words: string[] = [];
  const walk = (node: Node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent?.trim() ?? "";
      if (text !== "") words.push(text);
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;
    const el = node as Element;
    const tag = el.tagName.toLowerCase();
    if (tag === "script" || tag === "style" || tag === "template" || concealed(el)) return;
    if (tag === "select") {
      const chosen = (el as HTMLSelectElement).selectedOptions[0];
      if (chosen !== undefined && chosen.text.trim() !== "") words.push(chosen.text.trim());
      return;
    }
    const placeholder = el.getAttribute("placeholder");
    if (placeholder !== null && placeholder.trim() !== "") words.push(placeholder.trim());
    // The text of a text area is its value.
    if (tag === "textarea") return;
    for (const child of el.childNodes) walk(child);
  };
  walk(root);
  return words.join(" ");
}
