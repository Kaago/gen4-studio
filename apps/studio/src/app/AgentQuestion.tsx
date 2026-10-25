import type { AgentAnswer, QuestionDto } from "../api";
import { Dialog } from "../ui/primitives";
import type { DialogAction } from "../ui/primitives";
import styles from "./AgentQuestion.module.css";

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

/** Title, sentence and buttons of a question; the buttons stand in the order the decision names them, the last one is the main one. */
function contentOf(question: QuestionDto): { title: string; text: string; answers: readonly [string, AgentAnswer, DialogAction["kind"]][] } {
  const client = `„${question.client}“`;
  if (question.kind === "connect") {
    const project = question.project === null ? "das offene Projekt" : `das Projekt „${projectName(question.project)}“`;
    return {
      title: "Agent möchte zugreifen",
      text: `${client} möchte auf ${project} zugreifen.`,
      answers: [
        ["Ablehnen", "deny", "secondary"],
        ["Lesen", "read", "secondary"],
        ["Lesen und bearbeiten", "write", "primary"],
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
 * do the irreversible step. The hub waits for the answer (at most 120 s); closing the dialog in any way denies.
 */
export function AgentQuestion({ question, onAnswer }: AgentQuestionProps) {
  if (question === null) return null;
  const { title, text, answers } = contentOf(question);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onAnswer(question.id, "deny");
      }}
      title={title}
      actions={answers.map(([label, answer, kind]) => ({ label, kind, onClick: () => onAnswer(question.id, answer) }))}
    >
      <p className={styles.text} data-agent-question="">
        {text}
      </p>
    </Dialog>
  );
}
