import type { HeaderSummary, MatrixSummary, MemberCounts, WarpLink } from "../api";
import { NewAreaWizard } from "../workbench/wizards/NewAreaWizard";
import { areaResult } from "../workbench/wizards/wizardSpecs";
import type { Wizards } from "./useWizards";

interface Props {
  wizards: Pick<Wizards, "request" | "close" | "created">;
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
  links: WarpLink[];
  counts: MemberCounts;
  onError: (e: unknown) => void;
}

/**
 * The open wizard, above the workbench (a modal over everything). It lives beside the workbench,
 * not in it, so opening, typing and closing a wizard re-renders none of the workbench's panels.
 */
export function WizardHost({ wizards, headers, matrices, links, counts, onError }: Props) {
  const { request, close, created } = wizards;
  if (!request) return null;
  switch (request.kind) {
    case "area":
      return (
        <NewAreaWizard
          headers={headers}
          matrices={matrices}
          links={links}
          counts={counts}
          prefill={request.prefill}
          onClose={close}
          onCreated={(report, spec) => created(report.header, areaResult(report, spec).lines[0])}
          onError={onError}
        />
      );
    case "room":
      // The room wizard follows with its own task.
      return null;
  }
}
