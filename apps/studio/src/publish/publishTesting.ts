import type { PublishOptionsDto, PublishReportDto, ReleaseEntry, ReleasesDto } from "../api";

/** Test data for „Veröffentlichen“: a project `Sinnoh-Remix` at `/h/Sinnoh-Remix` (synthetic, no game's data). */

export const ROOT = "/h/Sinnoh-Remix";
export const BASE_SHA1 = "d40f19ac24d67c721a4ab25e1c013154443d75b4";
export const TARGET_SHA1 = "7e3a1b9c0d2f4e6a8b0c1d2e3f405162738495a6";

export const releaseOf = (over: Partial<ReleaseEntry> = {}): ReleaseEntry => ({
  version: "0.1.0",
  date: "2026-10-31T10:00:00Z",
  file: "build/Sinnoh-Remix-v0.1.0.xdelta",
  notes: "Neue Karte",
  baseSha1: BASE_SHA1,
  size: 4500,
  ...over,
});

/** The list as `releases` answers it: none yet unless given, the proposal `0.1.0`. */
export const releasesOf = (over: Partial<ReleasesDto> = {}): ReleasesDto => ({
  stem: "Sinnoh-Remix",
  next_version: "0.1.0",
  dir: `${ROOT}/build`,
  releases: [],
  ...over,
});

/** What `publishPatch` answers for `opts`: patch and notes (and `README.txt` with `readme`) in `opts.dir` (default `build/`). */
export function publishReportOf(opts: PublishOptionsDto, over: Partial<PublishReportDto> = {}): PublishReportDto {
  const dir = opts.dir ?? `${ROOT}/build`;
  const name = opts.file ?? `Sinnoh-Remix-v${opts.version}.xdelta`;
  const patch = `${dir}/${name.endsWith(".xdelta") ? name : `${name}.xdelta`}`;
  const release = releaseOf({ version: opts.version, notes: opts.notes ?? "", file: patch.startsWith(`${ROOT}/`) ? patch.slice(ROOT.length + 1) : patch });
  return {
    release,
    patch: { patch, notes: `${patch}.txt`, rom: `${ROOT}/build/Sinnoh-Remix-CPUD.nds`, size: 4500, base_sha1: BASE_SHA1, target_sha1: TARGET_SHA1, encode_ms: 120, verify_ms: 148 },
    files: [
      { kind: "patch", path: patch, size: 4500 },
      { kind: "notes", path: `${patch}.txt`, size: 600 },
      ...(opts.readme ? [{ kind: "readme" as const, path: `${dir}/README.txt`, size: 122 }] : []),
    ],
    releases: { version: 1, releases: [release] },
    phase_ms: { build: 69978, encode: 120, verify: 148, write: 3, record: 2 },
    ms: 70300,
    ...over,
  };
}
