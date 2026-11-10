import i18next from "i18next";
import type { FlatNamespace, ParseKeysWithoutVariables } from "i18next";
import { initReactI18next } from "react-i18next";
import commonDe from "./locales/de/common.json";
import appDe from "./locales/de/app.json";
import uiDe from "./locales/de/ui.json";
import workbenchDe from "./locales/de/workbench.json";
import worldDe from "./locales/de/world.json";
import dataDe from "./locales/de/data.json";
import storiesDe from "./locales/de/stories.json";
import flagsDe from "./locales/de/flags.json";
import technikDe from "./locales/de/technik.json";
import projectDe from "./locales/de/project.json";
import checksDe from "./locales/de/checks.json";
import gitDe from "./locales/de/git.json";
import publishDe from "./locales/de/publish.json";
import textprojectDe from "./locales/de/textproject.json";
import originalDe from "./locales/de/original.json";
import deleteDe from "./locales/de/delete.json";
import platformDe from "./locales/de/platform.json";
import recipesDe from "./locales/de/recipes.json";
import termsDe from "./locales/de/terms.json";
import commonEn from "./locales/en/common.json";
import appEn from "./locales/en/app.json";
import uiEn from "./locales/en/ui.json";
import workbenchEn from "./locales/en/workbench.json";
import worldEn from "./locales/en/world.json";
import dataEn from "./locales/en/data.json";
import storiesEn from "./locales/en/stories.json";
import flagsEn from "./locales/en/flags.json";
import technikEn from "./locales/en/technik.json";
import projectEn from "./locales/en/project.json";
import checksEn from "./locales/en/checks.json";
import gitEn from "./locales/en/git.json";
import publishEn from "./locales/en/publish.json";
import textprojectEn from "./locales/en/textproject.json";
import originalEn from "./locales/en/original.json";
import deleteEn from "./locales/en/delete.json";
import platformEn from "./locales/en/platform.json";
import recipesEn from "./locales/en/recipes.json";
import termsEn from "./locales/en/terms.json";

/** What the user picked: follow the system, or one language. */
export type Language = "system" | "de" | "en";
/** The language that is actually shown. */
export type Effective = "de" | "en";

/** Where the choice is remembered (like `g4.theme`). */
export const LANGUAGE_KEY = "g4.lang";
/** The namespace `t("key")` reads without a prefix. */
export const defaultNS = "common";

/**
 * Every catalog, one namespace per folder of `src`; `locales/<lang>/<ns>.json`. All namespaces are registered here from the
 * start: a folder that moves its texts fills its JSON files and never touches this list. `catalogs.test.ts` holds de and en equal.
 */
export const resources = {
  de: {
    common: commonDe,
    app: appDe,
    ui: uiDe,
    workbench: workbenchDe,
    world: worldDe,
    data: dataDe,
    stories: storiesDe,
    flags: flagsDe,
    technik: technikDe,
    project: projectDe,
    checks: checksDe,
    git: gitDe,
    publish: publishDe,
    textproject: textprojectDe,
    original: originalDe,
    delete: deleteDe,
    platform: platformDe,
    recipes: recipesDe,
    terms: termsDe,
  },
  en: {
    common: commonEn,
    app: appEn,
    ui: uiEn,
    workbench: workbenchEn,
    world: worldEn,
    data: dataEn,
    stories: storiesEn,
    flags: flagsEn,
    technik: technikEn,
    project: projectEn,
    checks: checksEn,
    git: gitEn,
    publish: publishEn,
    textproject: textprojectEn,
    original: originalEn,
    delete: deleteEn,
    platform: platformEn,
    recipes: recipesEn,
    terms: termsEn,
  },
} as const;

/** `de` for a navigator language starting with `de` (`de`, `de-AT`, `de_CH`, any case), else `en`. */
export function detectLanguage(navigatorLanguage: string): Effective {
  return /^de([-_]|$)/i.test(navigatorLanguage) ? "de" : "en";
}

/** The stored choice; no entry, an unknown value or a blocked storage mean `system`. */
export function readLanguage(): Language {
  let value: string | null = null;
  try {
    value = localStorage.getItem(LANGUAGE_KEY);
  } catch {
    /* no storage: the system decides */
  }
  return value === "de" || value === "en" ? value : "system";
}

/** The language shown for a choice; `system` follows the navigator. */
export function effectiveLanguage(l: Language = readLanguage()): Effective {
  if (l !== "system") return l;
  return detectLanguage(typeof navigator === "undefined" ? "" : navigator.language);
}

/** `<html lang>`; some tests run without a document. */
function setDocumentLanguage(lang: Effective): void {
  if (typeof document !== "undefined") document.documentElement.lang = lang;
}

/**
 * Starts i18next once, before the first render. A second call does nothing: the language changes through `applyLanguage`.
 * The resources are inline, so i18next is ready when this returns.
 */
export function init(l: Language): void {
  if (i18next.isInitialized) return;
  const lng = effectiveLanguage(l);
  void i18next.use(initReactI18next).init({
    lng,
    fallbackLng: "en",
    defaultNS,
    resources,
    interpolation: { escapeValue: false },
    returnNull: false,
  });
  setDocumentLanguage(lng);
}

/**
 * Makes `l` the language of the app: remembers it, switches i18next and `<html lang>`, then tells the backend (`set_locale`) so
 * its messages follow. Resolves when the backend has answered; a backend that refuses (an older one without the command) is
 * logged and does not keep the app from starting.
 */
export async function applyLanguage(l: Language): Promise<void> {
  try {
    localStorage.setItem(LANGUAGE_KEY, l);
  } catch {
    /* the choice is simply not remembered */
  }
  const effective = effectiveLanguage(l);
  await i18next.changeLanguage(effective);
  setDocumentLanguage(effective);
  try {
    // Loaded here, not at the top: `vitest.setup.ts` imports this module before every test, and the backend wrapper (with the
    // transport and Tauri's `invoke` under it) would be fixed to its real version before a test file's own `vi.mock` could replace it.
    const { setLocale } = await import("./backend");
    await setLocale(effective);
  } catch (e) {
    console.warn("set_locale failed; the backend keeps its language", e);
  }
}

/** For modules without React: `t("<ns>:<key>")`. Components use `useTranslation("<ns>")`. */
export const t = i18next.t;

/** A key `t` resolves without values: the default namespace's own keys and `"<ns>:<key>"` of every namespace. */
type SentenceKey = ParseKeysWithoutVariables<[typeof defaultNS, ...Exclude<FlatNamespace, typeof defaultNS>[]]>;

/** The languages the catalogs hold, in the order `matchesAnyLanguage` compares them. */
const LANGUAGES: readonly Effective[] = ["de", "en"];

/**
 * Whether `text` is the sentence of `key` (`"<ns>:<key>"`, without placeholders) in either language: for words that come from elsewhere —
 * the kernel or the backend, which speak the language they were set to, or a name given before the language switched — so a check holds
 * whichever language the app shows.
 */
export function matchesAnyLanguage(key: SentenceKey, text: unknown): boolean {
  return LANGUAGES.some((lng) => t(key, { lng }) === text);
}
