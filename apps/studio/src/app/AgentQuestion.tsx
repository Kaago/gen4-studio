import type { AgentAnswer, QuestionDto } from "../api";
import { useRef } from "react";
import { t } from "../i18n";
import { Dialog } from "../ui/primitives";
import type { DialogAction } from "../ui/primitives";

export type AgentQuestionProps = {
  /** The hub's open question; null: none, the dialog is closed. */
  question: QuestionDto | null;
  /** The answer to the question `id`. Escape, the close button and a click outside answer "dismiss" (a no that saves nothing). */
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
  const client = question.client;
  if (question.kind === "connect") {
    return {
      title: t("app:agentQuestion.connectTitle"),
      text:
        question.project === null
          ? t("app:agentQuestion.connectOpenProject", { client })
          : t("app:agentQuestion.connectProject", { client, project: projectName(question.project) }),
      answers: [
        [t("app:agentQuestion.deny"), "deny", "secondary"],
        [t("app:agentQuestion.read"), "read", "primary"],
        [t("app:agentQuestion.write"), "write", "secondary"],
      ],
    };
  }
  return {
    title: t("app:agentQuestion.confirmTitle"),
    text: t("app:agentQuestion.confirmText", { client, what: question.what ?? t("app:agentQuestion.somethingIrreversible") }),
    answers: [
      [t("app:agentQuestion.deny"), "deny", "secondary"],
      [t("app:agentQuestion.allow"), "allow", "danger"],
    ],
  };
}

/**
 * The question an agent's first call (or an irreversible step) puts to the user: may it read, read and edit, or not at all; or may it
 * do the irreversible step. The hub waits for the answer (at most 120 s); closing the dialog without a button denies this call only and
 * saves nothing (only „Ablehnen“ is saved as the project's „Nicht“). Each question is
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
        if (!open) answer("dismiss");
      }}
      title={title}
      description={text}
      actions={answers.map(([label, value, kind]) => ({ label, kind, onClick: () => answer(value) }))}
      data={{ "data-agent-question": "" }}
    />
  );
}
