// Helpers for the tests of the heads that show an id: the text project as a test sets it, around the part under test.
import type { ReactNode } from "react";
import { vi } from "vitest";
import type { Mock } from "vitest";
import type { RenameResult } from "../api";
import { EntityIdCache, TextProjectProvider } from "./TextProjectContext";

export type TextProjectSpy = { active: boolean; cache: EntityIdCache; onRenamed: Mock<(result: RenameResult) => void> };

/** A DSPRE folder unless `active`; `active` may be changed before the next render. */
export function textProjectSpy(active = false): TextProjectSpy {
  return { active, cache: new EntityIdCache(), onRenamed: vi.fn() };
}

/** `node` below the text project of `spy`. */
export function withTextProject(spy: TextProjectSpy, node: ReactNode): ReactNode {
  return (
    <TextProjectProvider active={spy.active} ids={spy.cache} onRenamed={spy.onRenamed}>
      {node}
    </TextProjectProvider>
  );
}
