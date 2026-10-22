// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const tauriInvoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriInvoke }));

import { BRIDGE_PREFIX, invoke } from "./transport";

const w = window as unknown as { __TAURI_INTERNALS__?: unknown };

const jsonResponse = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });

describe("platform/transport", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    tauriInvoke.mockReset();
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
    delete w.__TAURI_INTERNALS__;
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    delete w.__TAURI_INTERNALS__;
  });

  it("uses_tauri_inside_tauri", async () => {
    w.__TAURI_INTERNALS__ = {};
    tauriInvoke.mockResolvedValue(7);
    await expect(invoke<number>("get_header", { id: 415 })).resolves.toBe(7);
    expect(tauriInvoke).toHaveBeenCalledWith("get_header", { id: 415 });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("posts_json_to_the_bridge_outside_tauri", async () => {
    fetchMock.mockResolvedValue(jsonResponse(200, { id: 415 }));
    await expect(invoke("get_header", { id: 415 })).resolves.toEqual({ id: 415 });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(`${BRIDGE_PREFIX}/invoke/get_header`);
    expect(url).toBe("/__bridge/invoke/get_header");
    expect(init.method).toBe("POST");
    expect(init.body).toBe('{"id":415}');
    expect(init.headers).toEqual({ "Content-Type": "application/json" });
    expect(tauriInvoke).not.toHaveBeenCalled();
  });

  it("sends_an_empty_object_without_args", async () => {
    fetchMock.mockResolvedValue(jsonResponse(200, null));
    await expect(invoke("close_project")).resolves.toBeNull();
    expect((fetchMock.mock.calls[0] as [string, RequestInit])[1].body).toBe("{}");
  });

  it("returns_an_array_buffer_for_octet_stream", async () => {
    fetchMock.mockResolvedValue(
      new Response(new Uint8Array([1, 2, 3]), { status: 200, headers: { "Content-Type": "application/octet-stream" } }),
    );
    const out = await invoke<ArrayBuffer>("get_building_glb", { id: 1 });
    expect(out).toBeInstanceOf(ArrayBuffer);
    expect(Array.from(new Uint8Array(out))).toEqual([1, 2, 3]);
  });

  it("rejects_with_the_cmd_error_body", async () => {
    const body = { code: "stale", message: "Datei hat sich geändert", asset: "a.bin" };
    fetchMock.mockResolvedValue(jsonResponse(400, body));
    await expect(invoke("set_header", {})).rejects.toEqual(body);
  });

  it("rejects_with_http_status_without_body", async () => {
    fetchMock.mockResolvedValue(new Response("nope", { status: 404 }));
    await expect(invoke("nope")).rejects.toEqual({ code: "error", message: "Brücke: HTTP 404", asset: null });
  });

  it("rejects_with_http_status_for_non_cmd_error_json", async () => {
    fetchMock.mockResolvedValue(jsonResponse(500, { foo: 1 }));
    await expect(invoke("x")).rejects.toEqual({ code: "error", message: "Brücke: HTTP 500", asset: null });
  });

  it("unreachable_bridge_rejects_with_a_cmd_error", async () => {
    fetchMock.mockRejectedValue(new TypeError("Failed to fetch"));
    await expect(invoke("get_header", { id: 1 })).rejects.toEqual({
      code: "error",
      message: "Brücke nicht erreichbar",
      asset: null,
    });
  });
});
