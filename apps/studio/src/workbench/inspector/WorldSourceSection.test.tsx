// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { worldAdopt, worldBake, worldToBinary } from "../../api";
import type { WorldAdoptReport, WorldBakeReport } from "../../api";
import { resetSectionFolds } from "./foldState";
import { WorldSourceSection } from "./WorldSourceSection";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  worldAdopt: vi.fn(),
  worldBake: vi.fn(),
  worldToBinary: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const bake = (over: Partial<WorldBakeReport> = {}): WorldBakeReport => ({
  map: 0,
  ok: true,
  style: "outdoor",
  faces: 0,
  model_bytes: 21740,
  model_share: 0.35,
  model_kept: false,
  textures: [],
  vram_bytes: 0,
  bdhc_bytes: 350,
  bdhc_payload: 334,
  plates: 4,
  blocked: 0,
  verbatim_model: true,
  verbatim_terrain: true,
  messages: [],
  freeform_faces: 0,
  strips: 0,
  single_quads: 0,
  ...over,
});

const adopted = (deviations: number): WorldAdoptReport => ({
  map: 0,
  tileset: 6,
  tileset_guessed: false,
  deviations: Array.from({ length: deviations }, (_, i) => ({ x: i, z: 0, height: 4.5, step: 1 })),
  change: 1,
  bake: bake(),
  form2: null,
});

describe("WorldSourceSection", () => {
  let container: HTMLElement;
  let root: Root;
  const onChanged = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetSectionFolds();
    onChanged.mockReset();
    onError.mockReset();
    vi.mocked(worldBake).mockReset().mockResolvedValue(bake());
    vi.mocked(worldAdopt).mockReset().mockResolvedValue(adopted(12));
    vi.mocked(worldToBinary).mockReset().mockResolvedValue({ map: 0, tileset: 6, cells: 1024, change: 2, bake: null });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(world: boolean, textProject = true, revision = 0) {
    await act(async () =>
      root.render(<WorldSourceSection map={0} world={world} textProject={textProject} revision={revision} onChanged={onChanged} onError={onError} />),
    );
    await flush();
  }
  const section = () => container.querySelector<HTMLElement>("[data-world-source]");
  const adopt = () => container.querySelector<HTMLButtonElement>("[data-world-adopt]");
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name);
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());

  it("binary_offers_adopt_in_the_text_project", async () => {
    await show(false);
    expect(section()?.getAttribute("data-world-source")).toBe("binary");
    expect(section()?.querySelector("h3")?.textContent).toBe("Quelle");
    expect(section()?.textContent).toContain("Binäre Karte aus dem ROM oder aus DSPRE.");
    expect(adopt()?.textContent).toBe("Karte als Quelle übernehmen");
    expect(adopt()?.disabled).toBe(false);
    // A binary map has no bake to read.
    expect(worldBake).not.toHaveBeenCalled();
  });

  it("adopt_shows_the_deviations", async () => {
    await show(false);
    await click(adopt());
    expect(worldAdopt).toHaveBeenCalledExactlyOnceWith(0);
    expect(onChanged).toHaveBeenCalledExactlyOnceWith(0);
    expect(section()?.querySelector("[role=status]")?.textContent).toBe("Übernommen. 12 Kacheln weichen von der Höhe ab.");
    // The parent follows with the world: the notice stays.
    await show(true, true, 1);
    expect(section()?.getAttribute("data-world-source")).toBe("world");
    expect(section()?.textContent).toContain("Übernommen. 12 Kacheln weichen von der Höhe ab.");
    // ⌘Z: binary again, the notice goes with the world.
    await show(false, true, 2);
    expect(section()?.querySelector("[role=status]")).toBeNull();

    // No deviation: just „Übernommen.“; one: the singular.
    vi.mocked(worldAdopt).mockResolvedValue(adopted(0));
    await act(async () => root.render(<div />));
    await show(false);
    await click(adopt());
    expect(section()?.querySelector("[role=status]")?.textContent).toBe("Übernommen.");
    vi.mocked(worldAdopt).mockResolvedValue(adopted(1));
    await act(async () => root.render(<div />));
    await show(false);
    await click(adopt());
    expect(section()?.querySelector("[role=status]")?.textContent).toBe("Übernommen. 1 Kachel weicht von der Höhe ab.");

    // A refusal goes to the app's error line, nothing changed.
    const refusal = { code: "error", message: "Karte 0000 ist schon eine Welt-Karte.", asset: null };
    vi.mocked(worldAdopt).mockRejectedValue(refusal);
    onChanged.mockReset();
    await act(async () => root.render(<div />));
    await show(false);
    await click(adopt());
    expect(onError).toHaveBeenCalledExactlyOnceWith(refusal);
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("dspre_layout_disables_adopt", async () => {
    await show(false, false);
    expect(section()?.getAttribute("data-world-source")).toBe("binary");
    expect(adopt()?.disabled).toBe(true);
    const reason = document.getElementById(adopt()?.getAttribute("aria-describedby") ?? "");
    expect(reason?.textContent).toBe("Boden malen gibt es im Textprojekt (Technik › Textprojekt).");
    await click(adopt());
    expect(worldAdopt).not.toHaveBeenCalled();
  });

  it("world_shows_the_bake_and_its_refusal", async () => {
    await show(true);
    expect(section()?.getAttribute("data-world-source")).toBe("world");
    expect(section()?.textContent).toContain("Welt-Karte");
    expect(worldBake).toHaveBeenCalledExactlyOnceWith(0);
    const line = container.querySelector<HTMLElement>("[data-world-bake]");
    expect(line?.getAttribute("data-world-bake")).toBe("ok");
    expect(line?.textContent).toBe("Gebacken · Modell 21.740 B · Terrain 334 B");
    expect(button("Als binäre Karte übernehmen")).toBeDefined();
    expect(adopt()).toBeNull();

    // The same revision reads nothing again; a new one does.
    await show(true, true, 0);
    expect(worldBake).toHaveBeenCalledTimes(1);
    vi.mocked(worldBake).mockResolvedValue(
      bake({
        ok: false,
        model_kept: true,
        messages: [
          { code: "bake.vram-high", level: "warning", args: [], message: "VRAM über dem Vanilla-Höchstwert." },
          { code: "bake.model-too-big", level: "error", args: [], message: "Das Modell ist 70 000 Byte groß, erlaubt sind 61 440 Byte." },
          { code: "bake.empty", level: "error", args: [], message: "Zweite Meldung." },
        ],
      }),
    );
    await show(true, true, 1);
    expect(worldBake).toHaveBeenCalledTimes(2);
    const refused = container.querySelector<HTMLElement>("[data-world-bake]");
    expect(refused?.getAttribute("data-world-bake")).toBe("refused");
    expect(refused?.querySelector("[role=alert]")?.textContent).toBe("Das Modell ist 70 000 Byte groß, erlaubt sind 61 440 Byte.");

    // A world whose rows do not read: the command's refusal is the line.
    vi.mocked(worldBake).mockRejectedValue({ code: "error", message: "world.schema: Zeile 3 hat 31 Kacheln.", asset: null });
    await show(true, true, 2);
    const broken = container.querySelector<HTMLElement>("[data-world-bake]");
    expect(broken?.getAttribute("data-world-bake")).toBe("refused");
    expect(broken?.textContent).toBe("world.schema: Zeile 3 hat 31 Kacheln.");
  });

  it("to_binary_calls_the_command", async () => {
    await show(true);
    await click(button("Als binäre Karte übernehmen"));
    expect(worldToBinary).toHaveBeenCalledExactlyOnceWith(0);
    expect(onChanged).toHaveBeenCalledExactlyOnceWith(0);
    vi.mocked(worldToBinary).mockRejectedValue({ code: "error", message: "nein", asset: null });
    await click(button("Als binäre Karte übernehmen"));
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onChanged).toHaveBeenCalledTimes(1);
  });

  it("adopt_as_gltf_shows_the_polygon_counts", async () => {
    vi.mocked(worldAdopt).mockResolvedValue({
      ...adopted(0),
      bake: bake({ verbatim_model: false, faces: 1234 }),
      form2: {
        vanilla: { polygons: 815, quads: 813, triangles: 2, bytes: 21740 },
        baked: { faces: 1628, quads: 1600, triangles: 28, strips: 90, single_quads: 40, dropped: 0, bytes: 30000, untextured: 0 },
        ignored: [],
        glb: "0000.model.glb",
      },
    });
    await show(false);
    // Form 1 stays the first `[data-world-adopt]`; Form 2 follows it.
    const buttons = [...container.querySelectorAll<HTMLButtonElement>("[data-world-adopt]")];
    expect(buttons.map((b) => [b.dataset.worldAdopt, b.textContent])).toEqual([
      ["", "Karte als Quelle übernehmen"],
      ["gltf", "Als glTF übernehmen (Form 2)"],
    ]);
    await click(container.querySelector("[data-world-adopt='gltf']"));
    expect(worldAdopt).toHaveBeenCalledExactlyOnceWith(0, "gltf");
    expect(onChanged).toHaveBeenCalledExactlyOnceWith(0);
    expect(section()?.querySelector("[role=status]")?.textContent).toBe("Übernommen als glTF: 815 Vanilla-Polygone, 1.628 gebackene Flächen.");
    // Outside a text project Form 2 is locked like Form 1.
    await act(async () => root.render(<div />));
    await show(false, false);
    expect(container.querySelector<HTMLButtonElement>("[data-world-adopt='gltf']")?.disabled).toBe(true);
  });

  it("adopt_as_gltf_refusal_is_a_notice", async () => {
    const sentence = "Form 2 braucht apicula (`G4_APICULA` oder tools/apicula).";
    vi.mocked(worldAdopt).mockRejectedValue({ code: "error", message: sentence, asset: null });
    await show(false);
    await click(container.querySelector("[data-world-adopt='gltf']"));
    expect(worldAdopt).toHaveBeenCalledExactlyOnceWith(0, "gltf");
    expect(section()?.querySelector("[role=alert]")?.textContent).toBe(sentence);
    expect(onError).not.toHaveBeenCalled();
    expect(onChanged).not.toHaveBeenCalled();
    // Form 1 afterwards: the refusal goes.
    vi.mocked(worldAdopt).mockResolvedValue(adopted(0));
    await click(container.querySelector("[data-world-adopt='']"));
    expect(section()?.querySelector("[role=alert]")).toBeNull();
  });
});
