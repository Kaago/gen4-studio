import { describe, expect, it, vi } from "vitest";
import { getMapModelGlb } from "../api";
import { toolStatus } from "../app/toolStatus";
import { thumbnails } from "./thumbnailStore";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getMapModelGlb: vi.fn() }));
// No WebGL here: a picture is a fixed Data-URL.
vi.mock("./thumbnailRenderer", () => ({ createTopDownRenderer: () => async () => "data:image/png;base64," }));

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("thumbnails and apicula", () => {
  it("a_picture_says_apicula_works_and_a_missing_tool_says_it_is_missing_for_good", async () => {
    expect(toolStatus.get().apicula).toBe("unbekannt");
    vi.mocked(getMapModelGlb).mockResolvedValueOnce(new ArrayBuffer(4));
    thumbnails.request(1, 0);
    await settle();
    expect(thumbnails.get(1, 0)).toBe("data:image/png;base64,");
    expect(toolStatus.get().apicula).toBe("bereit");

    vi.mocked(getMapModelGlb).mockRejectedValueOnce({ code: "tool_missing", message: "apicula fehlt", asset: null });
    thumbnails.request(2, 0);
    await settle();
    expect(toolStatus.get().apicula).toBe("fehlt");

    // A model from the backend's cache comes back while apicula is missing: it stays missing.
    vi.mocked(getMapModelGlb).mockResolvedValueOnce(new ArrayBuffer(4));
    thumbnails.request(3, 0);
    await settle();
    expect(thumbnails.get(3, 0)).toBe("data:image/png;base64,");
    expect(toolStatus.get().apicula).toBe("fehlt");
  });
});
