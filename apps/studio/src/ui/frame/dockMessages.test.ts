// @vitest-environment node
import { describe, expect, it } from "vitest";
import { messages } from "./dockMessages";

describe("dock messages", () => {
  it("says_floated_in_german", () => {
    expect(messages.groupFloated!("Inspector")).toBe("Inspector schwebt");
  });

  it("says_opened_and_closed_in_german", () => {
    expect(messages.panelOpened!("Script")).toBe("Script geöffnet");
    expect(messages.panelClosed!("Script")).toBe("Script geschlossen");
  });

  it("says_maximized_and_restored_in_german", () => {
    expect(messages.groupMaximized!("3D")).toBe("3D maximiert");
    expect(messages.groupRestored!("3D")).toBe("3D wiederhergestellt");
  });

  it("says_docked_and_popped_out_in_german", () => {
    expect(messages.groupDocked!("Inspector")).toBe("Inspector angedockt");
    expect(messages.groupPoppedOut!("Inspector")).toBe("Inspector in eigenem Fenster");
  });

  it("names_the_close_button_in_german", () => {
    expect(messages.closeTab!("Probleme")).toBe("Probleme schließen");
    expect(messages.closeTabPlain!()).toBe("Schließen");
  });

  it("leaves_the_keyboard_docking_texts_unset", () => {
    // The keyboard docking (Ctrl+M) is not switched on; its narration keeps dockview's defaults.
    for (const key of ["movePickTarget", "movePickEdge", "moveCommitted", "moveCancelled", "moveNotAllowed", "moveFloated"]) expect(messages).not.toHaveProperty(key);
  });
});
