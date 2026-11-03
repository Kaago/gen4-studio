// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RewardOnceForm } from "./RewardOnceForm";
import { inputOf, rewardSays } from "./storyRecipeModel";
import type { StoryRecipeForm } from "./storyRecipeModel";

type Reward = Extract<StoryRecipeForm, { recipe: "reward_once" }>;

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), freeFlag: vi.fn(), validateText: vi.fn() }));
vi.mock("../../data/useDataArea", () => ({ useDataArea: () => ({ names: { items: ["", "Trank", "Top-Trank"] } }) }));
// The picker itself needs the data store: a button that picks item 1 stands in.
vi.mock("../../data/pokedex/pickers", () => ({
  PickField: ({ onPick, label }: { onPick: (n: number) => void; label: string }) => (
    <button type="button" data-pick={label} onClick={() => onPick(1)}>
      wählen
    </button>
  ),
}));
vi.mock("./FigureField", () => ({ FigureField: () => null }));
vi.mock("./StepField", () => ({ StepField: () => null }));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const start: Reward = { recipe: "reward_once", story: "s", figure: { header: 3, local_id: 1 }, step: 2, item: null, itemName: null, text: null, flag: 7, stepName: null };
let latest: Reward = start;

function Harness() {
  const [form, setForm] = useState<Reward>(start);
  latest = form;
  return <RewardOnceForm form={form} onChange={setForm} steps={[]} headers={[]} project={null} revision={0} onError={vi.fn()} />;
}

describe("RewardOnceForm", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(async () => {
    const api = await import("../../api");
    vi.mocked(api.freeFlag).mockReset().mockResolvedValue({ number: 7, pool_free: 3 } as never);
    vi.mocked(api.validateText).mockReset().mockResolvedValue([]);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const field = () => container.querySelector<HTMLTextAreaElement>("textarea")!;

  it("a_reward_says_du_erhaeltst_until_a_text_is_typed", async () => {
    await act(async () => root.render(<Harness />));
    await flush();
    // The empty field's mount echo is no typed text.
    expect(latest.text).toBeNull();
    await act(async () => container.querySelector<HTMLButtonElement>("[data-pick=Gegenstand]")!.click());
    await flush();
    expect(latest.item).toBe(1);
    expect(latest.text).toBeNull();
    expect(field().value).toBe("Du erhältst Trank!");
    expect(rewardSays(latest)).toBe("Du erhältst Trank!");
    // So the recipe has its input: „Anlegen“ no longer waits for a typed text.
    expect(inputOf(latest, { headers: [] })).toMatchObject({ item: 1, text: "Du erhältst Trank!" });
  });

  it("a_typed_text_replaces_the_default", async () => {
    await act(async () => root.render(<Harness />));
    await flush();
    await act(async () => container.querySelector<HTMLButtonElement>("[data-pick=Gegenstand]")!.click());
    await flush();
    const set = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!;
    await act(async () => {
      set.call(field(), "Nimm das!");
      field().dispatchEvent(new Event("input", { bubbles: true }));
    });
    await flush();
    expect(latest.text).toBe("Nimm das!");
    expect(field().value).toBe("Nimm das!");
  });
});
