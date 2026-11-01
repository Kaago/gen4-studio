import type { ThreadGraphDto } from "../../api.stories";

/**
 * A small thread with a story on it, for the gallery and the tests: a script sets 1 and 2, an event checks 1, a level script checks 2,
 * a step names 3 that the game does not (noted only), and a script uses the variable without a value (loose).
 */
export const sampleThreadGraph: ThreadGraphDto = {
  var: 0x4031,
  name: "VAR_UNK_0x4031",
  own_id: "VAR_RIVALIN",
  nodes: [
    {
      kind: "value",
      id: "v:1",
      value: 1,
      step: { key: "s1", value: 1, kind: "dialog", name: "Rivalin trifft" },
      noted_only: false,
    },
    { kind: "value", id: "v:2", value: 2, step: { key: "s2", value: 2, kind: "battle", name: "Kampf am See" }, noted_only: false },
    { kind: "value", id: "v:3", value: 3, step: { key: "s3", value: 3, kind: "reward", name: "Pokédex bekommen" }, noted_only: true },
    { kind: "value", id: "v:4", value: 4, step: null, noted_only: false },
    {
      kind: "place",
      id: "p:script:0101",
      source: { kind: "Script", index: 101 },
      header: 411,
      header_name: "Zweiblattdorf",
      label: "Scriptdatei 0101",
      loose: false,
      nav: { kind: "script", file: 101, line: 12 },
    },
    {
      kind: "place",
      id: "p:event:0394",
      source: { kind: "Event", index: 394 },
      header: 412,
      header_name: "Route 201",
      label: "Eventdatei 0394",
      loose: false,
      nav: { kind: "event", eventFile: 394, eventKind: "coord", index: 0, header: 412 },
    },
    {
      kind: "place",
      id: "p:script:0005",
      source: { kind: "Script", index: 5 },
      header: null,
      header_name: null,
      label: "Level-Script 0005",
      loose: false,
      nav: { kind: "script", file: 5 },
    },
    {
      kind: "place",
      id: "p:script:0200",
      source: { kind: "Script", index: 200 },
      header: null,
      header_name: null,
      label: "Scriptdatei 0200",
      loose: true,
      nav: { kind: "script", file: 200, line: 3 },
    },
  ],
  edges: [
    { from: "p:script:0101", to: "v:1", value: 1, access: "write", field: "SetVar", slot: 12 },
    { from: "p:script:0101", to: "v:2", value: 2, access: "write", field: "SetVar", slot: 40 },
    { from: "p:script:0101", to: "v:4", value: 4, access: "write", field: "SetVar", slot: 57 },
    { from: "v:1", to: "p:event:0394", value: 1, access: "read", field: "var", slot: 0 },
    { from: "v:2", to: "p:script:0005", value: 2, access: "read", field: "var_conditions", slot: 0 },
  ],
  layout: null,
  ms: 3,
};
