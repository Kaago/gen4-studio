// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { INFO_MS, listToasts, pauseToast, resetToasts, resumeToast, subscribeToasts, toast } from "./toasts";

describe("toast store", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    resetToasts();
  });
  afterEach(() => {
    resetToasts();
    vi.useRealTimers();
  });

  it("info_disappears_after_five_seconds_and_error_stays", () => {
    toast.info("Gespeichert");
    const errorId = toast.error("Das Speichern ist fehlgeschlagen.");
    expect(listToasts()).toHaveLength(2);
    vi.advanceTimersByTime(INFO_MS - 1);
    expect(listToasts()).toHaveLength(2);
    vi.advanceTimersByTime(1);
    expect(listToasts().map((t) => t.id)).toEqual([errorId]);
    vi.advanceTimersByTime(60_000);
    expect(listToasts().map((t) => t.id)).toEqual([errorId]);
  });

  it("an_error_with_actions_survives_the_five_seconds", () => {
    const onClick = vi.fn();
    toast.error("Build abgebrochen", { actions: [{ label: "Probleme zeigen", onClick }] });
    vi.advanceTimersByTime(INFO_MS * 3);
    const [only] = listToasts();
    expect(only?.kind).toBe("error");
    expect(only?.actions.map((a) => a.label)).toEqual(["Probleme zeigen"]);
  });

  it("progress_with_the_same_key_replaces_instead_of_stacking", () => {
    toast.progress("load", "1/3");
    toast.progress("load", "2/3");
    expect(listToasts()).toHaveLength(1);
    expect(listToasts()[0]?.text).toBe("2/3");
    toast.progress("other", "Bericht");
    expect(listToasts()).toHaveLength(2);
    toast.progress("load", null);
    expect(listToasts().map((t) => t.text)).toEqual(["Bericht"]);
    toast.progress("load", null);
    expect(listToasts()).toHaveLength(1);
  });

  it("progress_stays_until_it_is_removed", () => {
    toast.progress("load", "1/3");
    vi.advanceTimersByTime(INFO_MS * 3);
    expect(listToasts()).toHaveLength(1);
  });

  it("keeps_at_most_four_and_drops_the_oldest_info_first", () => {
    toast.error("Fehler A");
    const oldestInfo = toast.info("Info 1");
    toast.info("Info 2");
    toast.info("Info 3");
    toast.info("Info 4");
    expect(listToasts().map((t) => t.text)).toEqual(["Fehler A", "Info 2", "Info 3", "Info 4"]);
    expect(listToasts().some((t) => t.id === oldestInfo)).toBe(false);
  });

  it("never_drops_an_error_while_an_info_exists", () => {
    toast.info("Info");
    toast.error("Fehler 1");
    toast.error("Fehler 2");
    toast.error("Fehler 3");
    toast.error("Fehler 4");
    expect(listToasts().map((t) => t.text)).toEqual(["Fehler 1", "Fehler 2", "Fehler 3", "Fehler 4"]);
    toast.error("Fehler 5");
    expect(listToasts().map((t) => t.text)).toEqual(["Fehler 2", "Fehler 3", "Fehler 4", "Fehler 5"]);
  });

  it("hover_pauses_the_clock_and_leaving_resumes_with_the_rest", () => {
    const id = toast.info("Gespeichert");
    vi.advanceTimersByTime(3000);
    pauseToast(id);
    vi.advanceTimersByTime(60_000);
    expect(listToasts()).toHaveLength(1);
    resumeToast(id);
    vi.advanceTimersByTime(1999);
    expect(listToasts()).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(listToasts()).toHaveLength(0);
  });

  it("dismiss_removes_one_toast_and_ignores_unknown_ids", () => {
    const a = toast.error("A");
    toast.error("B");
    toast.dismiss(a);
    toast.dismiss("nope");
    expect(listToasts().map((t) => t.text)).toEqual(["B"]);
  });

  it("subscribers_hear_every_change_and_the_list_keeps_its_identity_in_between", () => {
    const cb = vi.fn();
    const off = subscribeToasts(cb);
    toast.error("A");
    expect(cb).toHaveBeenCalledTimes(1);
    const before = listToasts();
    expect(listToasts()).toBe(before);
    toast.dismiss("nope");
    expect(cb).toHaveBeenCalledTimes(1);
    off();
    toast.error("B");
    expect(cb).toHaveBeenCalledTimes(1);
  });
});
