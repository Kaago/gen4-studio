import { Badge, PanelHead } from "../../ui/primitives";

export interface InspectorHeadProps {
  title: string;
  subtitle?: string;
  /** The record has changes that are not saved yet. */
  unsaved?: boolean;
  /** The record cannot be changed (a read-only event file). */
  locked?: boolean;
  /** The hover views: they show a record and cannot change it. */
  viewOnly?: boolean;
}

/** The head every inspector view shares: title, subtitle and the state of the record as badges. */
export function InspectorHead({ title, subtitle, unsaved, locked, viewOnly }: InspectorHeadProps) {
  const badges = unsaved || locked || viewOnly;
  return (
    <PanelHead
      title={title}
      subtitle={subtitle}
      actions={
        badges ? (
          <>
            {unsaved && <Badge kind="unsaved" />}
            {locked && <Badge kind="locked" />}
            {viewOnly && <Badge kind="info">Nur Ansicht</Badge>}
          </>
        ) : undefined
      }
    />
  );
}
