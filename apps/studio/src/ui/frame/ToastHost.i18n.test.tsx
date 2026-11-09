// @vitest-environment jsdom
import i18next from "i18next";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { pokemonTypes, typeName } from "../primitives";
import { messages } from "./dockMessages";
import { ToastHost } from "./ToastHost";
import { groupName } from "./Palette";
import { ago } from "./timeAgo";
import { resetToasts, toast } from "./toasts";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ToastHost in English", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(async () => {
    await i18next.changeLanguage("en");
    resetToasts();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<ToastHost />));
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
    // The suite runs in German (vitest.setup.ts).
    await i18next.changeLanguage("de");
  });

  it("names_the_region_and_the_close_button_in_english", () => {
    act(() => {
      toast.error("The file is missing.");
    });
    expect(container.querySelector("section")?.getAttribute("aria-label")).toBe("Notifications");
    expect(container.querySelector("button[aria-label=Close]")).not.toBeNull();
  });

  it("says_the_dock_titles_in_english", () => {
    expect(messages.groupFloated!("Inspector")).toBe("Inspector floated");
    expect(messages.closeTab!("Problems")).toBe("Close Problems");
    expect(messages.closeTabPlain!()).toBe("Close");
  });
});

describe("ui helpers in English", () => {
  beforeEach(async () => {
    await i18next.changeLanguage("en");
  });
  afterEach(async () => {
    await i18next.changeLanguage("de");
  });

  it("reads_the_texts_of_helpers_when_asked_not_when_loaded", () => {
    expect(ago(0, 4 * 60_000)).toBe("4 minutes ago");
    expect(ago(0, 60_000)).toBe("1 minute ago");
    expect(typeName(1)).toBe("Fighting");
    expect(typeName(19)).toBe("Type 19");
    expect(pokemonTypes[17]!.name).toBe("Dark");
    expect(groupName("Befehle")).toBe("Commands");
    expect(groupName("Orte")).toBe("Places");
  });
});
