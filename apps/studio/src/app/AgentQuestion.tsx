import type { AgentAnswer, QuestionDto } from "../api";
import { useRef } from "react";
import { Dialog } from "../ui/primitives";
import type { DialogAction } from "../ui/primitives";

export type AgentQuestionProps = {
  /** The hub's open question; null: none, the dialog is closed. */
  question: QuestionDto | null;
  /** The answer to the question `id`. Escape, the close button and a click outside answer "deny". */
  onAnswer: (id: number, answer: AgentAnswer) => void;
};

/** The project's folder name: the last part of its path, or the string itself when it has no slash. */
export function projectName(project: string): string {
  const parts = project.split(/[\\/]/).filter((part) => part !== "");
  return parts[parts.length - 1] ?? project;
}

/**
 * Title, sentence and buttons of a question, the buttons in the order the decision names them. Connecting, the safer „Lesen“ is the
 * main button; confirming, „Zulassen“ carries the danger colour.
 */
function contentOf(question: QuestionDto): { title: string; text: string; answers: readonly [string, AgentAnswer, DialogAction["kind"]][] } {
  const client = `„${question.client}“`;
  if (question.kind === "connect") {
    const project = question.project === null ? "das offene Projekt" : `das Projekt „${projectName(question.project)}“`;
    return {
      title: "Agent möchte zugreifen",
      text: `${client} möchte auf ${project} zugreifen.`,
      answers: [
        ["Ablehnen", "deny", "secondary"],
        ["Lesen", "read", "primary"],
        ["Lesen und bearbeiten", "write", "secondary"],
      ],
    };
  }
  return {
    title: "Agent möchte etwas Unumkehrbares tun",
    text: `${client} möchte: ${question.what ?? "etwas Unumkehrbares"} (lässt sich nicht rückgängig machen).`,
    answers: [
      ["Ablehnen", "deny", "secondary"],
      ["Zulassen", "allow", "danger"],
    ],
  };
}

/**
 * The question an agent's first call (or an irreversible step) puts to the user: may it read, read and edit, or not at all; or may it
 * do the irreversible step. The hub waits for the answer (at most 120 s); closing the dialog in any way denies. Each question is
 * answered once: a second click or the Escape of the closing dialog after an answer sends nothing; a new question gets a fresh dialog.
 */
export function AgentQuestion({ question, onAnswer }: AgentQuestionProps) {
  const answered = useRef<number | null>(null);
  if (question === null) return null;
  const { title, text, answers } = contentOf(question);
  const answer = (value: AgentAnswer) => {
    if (answered.current === question.id) return;
    answered.current = question.id;
    onAnswer(question.id, value);
  };
  return (
    <Dialog
      key={question.id}
      open
      onOpenChange={(open) => {
        if (!open) answer("deny");
      }}
      title={title}
      description={text}
      actions={answers.map(([label, value, kind]) => ({ label, kind, onClick: () => answer(value) }))}
      data={{ "data-agent-question": "" }}
    />
  );
}
