// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { MelonDsConfigDto, MelonDsDto } from "../../api.live";
import { LiveRow } from "./LiveRow";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const CONFIG = "/Users/p/Library/Preferences/melonDS/melonDS.toml";
const melonds = (config: MelonDsConfigDto | null): MelonDsDto => ({
  bundle: "/Applications/melonDS.app",
  binary: "/Applications/melonDS.app/Contents/MacOS/melonDS",
  version: "1.1",
  config_path: CONFIG,
  config,
});
const ready: MelonDsConfigDto = { enabled: true, break_on_startup: false, port: 3333, jit: false };

describe("LiveRow", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (m: MelonDsDto | null | undefined) => act(() => root.render(<LiveRow melonds={m} />));

  it("says_version_stub_and_jit_state_or_what_is_missing", () => {
    show(melonds(ready));
    expect(container.textContent).toContain("Spielmodus live");
    expect(container.textContent).toContain("melonDS 1.1 · GDB-Stub an · Port 3333 · JIT aus");
    expect(container.textContent).not.toContain("wird beim nächsten");
    // The path of the configuration, with itself as tooltip.
    expect(container.textContent).toContain(CONFIG);
    expect(container.querySelector(`[title="${CONFIG}"]`)).not.toBeNull();

    show(melonds({ ...ready, enabled: false }));
    expect(container.textContent).toContain("GDB-Stub aus: wird beim nächsten „Hier spielen“ eingeschaltet, wenn melonDS nicht läuft");
    show(melonds({ ...ready, jit: true }));
    expect(container.textContent).toContain("JIT an: wird beim nächsten „Hier spielen“ ausgeschaltet, wenn melonDS nicht läuft");
    show(melonds({ ...ready, break_on_startup: true }));
    expect(container.textContent).toContain("Halt beim Start an: wird beim nächsten „Hier spielen“ ausgeschaltet, wenn melonDS nicht läuft");
    show(melonds(null));
    expect(container.textContent).toContain("melonDS 1.1");
    expect(container.textContent).toContain("Einstellungen nicht lesbar");

  });

  it("says_not_yet_checked_while_an_emulator_is_chosen_and_no_melonds_only_without_one", () => {
    // The hub reads melonDS at „Hier spielen“: before that, a chosen emulator is not „none“.
    for (const none of [null, undefined]) {
      act(() => root.render(<LiveRow melonds={none} emulator="/Applications/melonDS.app" />));
      expect(container.textContent).toContain("Noch nicht geprüft: wird beim nächsten „Hier spielen“ gelesen");
      expect(container.textContent).not.toContain("Kein melonDS gewählt");
    }
    for (const emulator of [null, undefined, ""]) {
      act(() => root.render(<LiveRow melonds={null} emulator={emulator} />));
      expect(container.textContent).toContain("Kein melonDS gewählt");
      expect(container.textContent).not.toContain("Noch nicht geprüft");
    }
    // Once read, melonDS's own state wins.
    act(() => root.render(<LiveRow melonds={melonds(ready)} emulator={null} />));
    expect(container.textContent).toContain("melonDS 1.1 · GDB-Stub an · Port 3333 · JIT aus");
  });
});
