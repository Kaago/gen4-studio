// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";

const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("./transport", () => ({ inTauri: () => env.tauri }));

import { isMacTauri } from "./os";

const platform = (value: string) => Object.defineProperty(navigator, "platform", { value, configurable: true });

describe("isMacTauri", () => {
  afterEach(() => {
    env.tauri = false;
    Reflect.deleteProperty(navigator, "platform");
  });

  it("is_true_only_inside_tauri_on_a_mac", () => {
    platform("MacIntel");
    env.tauri = true;
    expect(isMacTauri()).toBe(true);
    env.tauri = false;
    expect(isMacTauri()).toBe(false);
  });

  it("is_false_inside_tauri_on_other_systems", () => {
    env.tauri = true;
    platform("Win32");
    expect(isMacTauri()).toBe(false);
    platform("Linux x86_64");
    expect(isMacTauri()).toBe(false);
  });
});
