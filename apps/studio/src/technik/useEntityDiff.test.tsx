// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { diffEntity } from "../api";
import type { Against, DiffSubject, EntityDiffReport } from "../api";
import { diffReport } from "./diffTesting";
import { useEntityDiff } from "./useEntityDiff";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  diffEntity: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const TRAINER: DiffSubject = { kind: "trainer", index: 1 };
const MAP: DiffSubject = { kind: "map", index: 3 };

describe("useEntityDiff", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: ReturnType<typeof useEntityDiff>;

  type ProbeProps = { subject: DiffSubject | null; against?: Against; revision?: number; headId?: string | null };
  function Probe({ subject, against = "head", revision = 0, headId = null }: ProbeProps) {
    shown = useEntityDiff(subject, against, revision, headId);
    return null;
  }
  const show = (props: ProbeProps) => act(async () => root.render(<Probe {...props} />));

  beforeEach(() => {
    vi.mocked(diffEntity).mockReset().mockResolvedValue(diffReport());
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => act(() => root.unmount()));

  it("reads_on_selection_and_after_a_revision", async () => {
    // Nothing selected: nothing read.
    await show({ subject: null });
    expect(diffEntity).not.toHaveBeenCalled();
    expect(shown).toEqual({ report: null, error: null, busy: false });
    // Selected: read against the comparison.
    await show({ subject: TRAINER });
    expect(diffEntity).toHaveBeenCalledExactlyOnceWith(TRAINER, "head");
    expect(shown.report?.label).toBe("Trainer 0001");
    expect(shown.busy).toBe(false);
    // The same subject as a new object (a new list) reads nothing again.
    await show({ subject: { ...TRAINER } });
    expect(diffEntity).toHaveBeenCalledOnce();
    // The saved project moved: read again.
    await show({ subject: TRAINER, revision: 1 });
    expect(diffEntity).toHaveBeenCalledTimes(2);
    // Another comparison: read again, against it.
    await show({ subject: TRAINER, against: "saved", revision: 1 });
    expect(diffEntity).toHaveBeenLastCalledWith(TRAINER, "saved");
    // A refusal: its sentence, no report.
    vi.mocked(diffEntity).mockRejectedValueOnce({ code: "no_commit", message: "Noch kein Commit.", asset: null });
    await show({ subject: MAP, against: "saved", revision: 1 });
    expect(shown).toEqual({ report: null, error: "Noch kein Commit.", busy: false });
    // Nothing selected again: nothing shown.
    await show({ subject: null });
    expect(shown).toEqual({ report: null, error: null, busy: false });
  });

  it("reads_again_when_head_moves_against_the_last_commit", async () => {
    await show({ subject: TRAINER, headId: "a".repeat(40) });
    expect(diffEntity).toHaveBeenCalledOnce();
    // A commit made outside: the same subject is read against the new `HEAD`.
    await show({ subject: TRAINER, headId: "b".repeat(40) });
    expect(diffEntity).toHaveBeenCalledTimes(2);
    // Against the saved state `HEAD` does not matter.
    await show({ subject: TRAINER, against: "saved", headId: "b".repeat(40) });
    expect(diffEntity).toHaveBeenCalledTimes(3);
    await show({ subject: TRAINER, against: "saved", headId: "c".repeat(40) });
    expect(diffEntity).toHaveBeenCalledTimes(3);
  });

  it("drops_an_answer_for_an_older_selection", async () => {
    let answerTrainer: (r: EntityDiffReport) => void = () => {};
    vi.mocked(diffEntity).mockImplementationOnce(() => new Promise((resolve) => (answerTrainer = resolve)));
    await show({ subject: TRAINER });
    expect(shown.busy).toBe(true);
    expect(shown.report).toBeNull();
    // The map is chosen while the trainer is still read.
    let answerMap: (r: EntityDiffReport) => void = () => {};
    vi.mocked(diffEntity).mockImplementationOnce(() => new Promise((resolve) => (answerMap = resolve)));
    await show({ subject: MAP });
    await act(async () => answerMap(diffReport({ subject: MAP, label: "Karte 0003" })));
    expect(shown.report?.label).toBe("Karte 0003");
    // The trainer's late answer changes nothing.
    await act(async () => answerTrainer(diffReport()));
    expect(shown.report?.label).toBe("Karte 0003");
    expect(shown.busy).toBe(false);
  });
});
