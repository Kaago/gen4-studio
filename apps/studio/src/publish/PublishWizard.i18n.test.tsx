// @vitest-environment jsdom
import i18next from "i18next";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PublishWizard } from "./PublishWizard";
import { publishReportOf, releasesOf } from "./publishTesting";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("PublishWizard in English", () => {
  let container: HTMLElement;
  let root: Root;

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    await act(() => i18next.changeLanguage("de"));
  });

  async function show() {
    await act(() => i18next.changeLanguage("en"));
    await act(async () =>
      root.render(
        <PublishWizard
          defaults={releasesOf()}
          defaultsError={null}
          gitOwn
          onPublish={vi.fn(async (opts) => publishReportOf(opts))}
          onReveal={vi.fn(async () => {})}
          onCommit={vi.fn()}
          onClose={vi.fn()}
        />,
      ),
    );
  }

  it("shows_the_title_and_the_version_step_in_English", async () => {
    await show();
    const text = container.textContent ?? "";
    // The title of the frame and the first step; the German words are gone.
    expect(text).toContain("Publish");
    expect(text).toContain("Version");
    expect(text).not.toContain("Veröffentlichen");
    expect(text).not.toContain("Änderungsnotizen");
    expect(container.querySelector('[role="dialog"]')?.getAttribute("aria-labelledby")).not.toBeNull();
    expect(container.querySelector('[aria-label="Version"]')).not.toBeNull();
    expect(container.querySelector('[aria-label="Change notes"]')).not.toBeNull();
    expect(container.querySelector('[aria-label="File name"]')).not.toBeNull();
  });
});
