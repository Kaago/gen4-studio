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
  /** The matrix of the selected header: the area wizard copies from it unless the request names another. */
  selectedMatrix: number | null;
  onError: (e: unknown) => void;
}

/**
 * The open wizard, above the workbench (a modal over everything). It lives beside the workbench,
 * not in it, so opening, typing and closing a wizard re-renders none of the workbench's panels.
 */
export function WizardHost({ wizards, headers, matrices, links, counts, selectedMatrix, onError }: Props) {
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
          prefill={{ ...request.prefill, matrix: request.prefill?.matrix ?? selectedMatrix ?? undefined }}
          onClose={close}
          onCreated={(report, spec) => created({ matrix: report.matrix, header: report.header, announcement: areaResult(report, spec).lines[0] })}
          onError={onError}
        />
      );
    case "room":
      // The room wizard follows with its own task.
      return null;
  }
}
