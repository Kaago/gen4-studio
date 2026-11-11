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

/** How long the start waits for the backend's system language before `navigator.language` decides. */
export const SYSTEM_LANGUAGE_TIMEOUT_MS = 1500;

/** How long a switch waits for the backend to take the new language (`set_locale`) before the app switches anyway. */
export const SET_LOCALE_TIMEOUT_MS = 3000;

/** The system language as the backend reported it (inside Tauri), kept from the first answer on; `null`: the navigator decides. */
let systemLanguage: Effective | null = null;
/** The one question to the backend, shared by all who wait for it. */
let systemAsked: Promise<Effective> | null = null;

/** `navigator.language` as a language; some tests run without a navigator. */
function navigatorLanguage(): Effective {
  return detectLanguage(typeof navigator === "undefined" ? "" : navigator.language);
}

/**
 * The language shown for a choice; `system` follows the system language the backend reported (inside Tauri, see `resolveSystemLanguage`),
 * else the navigator.
 */
export function effectiveLanguage(l: Language = readLanguage()): Effective {
  if (l !== "system") return l;
  return systemLanguage ?? navigatorLanguage();
}

/**
 * Asks the backend once for the language of the operating system (`system_language`) and keeps the answer for `effectiveLanguage`: inside
 * Tauri `navigator.language` may report the bundle's localizations rather than the OS language (WKWebView can name „en-US“ on a German
 * Mac). Outside Tauri there is nothing to ask; there, and when the backend fails, reports nothing or does not answer within `timeoutMs`, the navigator
 * decides (an answer after the timeout is dropped: the language shown must not change behind the user's back). Resolves to what
 * `effectiveLanguage("system")` answers from then on.
 */
export function resolveSystemLanguage(timeoutMs: number = SYSTEM_LANGUAGE_TIMEOUT_MS): Promise<Effective> {
  systemAsked ??= askSystemLanguage(timeoutMs);
  return systemAsked;
}

async function askSystemLanguage(timeoutMs: number): Promise<Effective> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    // Loaded lazily for the reason given in `applyLanguage`.
    const backend = await import("./backend");
    const timeout = new Promise<null>((resolve) => {
      timer = setTimeout(() => resolve(null), timeoutMs);
    });
    const reported = await Promise.race([backend.systemLanguage(), timeout]);
    if (reported) systemLanguage = detectLanguage(reported);
  } catch (e) {
    console.warn("system_language failed; the navigator language decides", e);
  } finally {
    clearTimeout(timer);
  }
  return effectiveLanguage("system");
}

/** For the tests: forgets the backend's answer, the next `resolveSystemLanguage` asks again. */
export function forgetSystemLanguage(): void {
  systemLanguage = null;
  systemAsked = null;
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

/** The number of the latest `applyLanguage` call: an earlier one that is still waiting for the backend leaves the switch to the later. */
let latestApply = 0;

/**
 * Tells the backend the language (`set_locale`) and waits for its answer, at most `SET_LOCALE_TIMEOUT_MS`. A refusal (an older backend
 * without the command) and a missing answer are logged once and do not keep the app from switching.
 */
async function tellBackend(lang: Effective): Promise<void> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const timeout = new Promise<never>((_, reject) => {
      timer = setTimeout(() => reject(new Error(`no answer within ${SET_LOCALE_TIMEOUT_MS} ms`)), SET_LOCALE_TIMEOUT_MS);
    });
    const told = (async () => {
      // Loaded here, not at the top: `vitest.setup.ts` imports this module before every test, and the backend wrapper (with the
      // transport and Tauri's `invoke` under it) would be fixed to its real version before a test file's own `vi.mock` could replace it.
      const { setLocale } = await import("./backend");
      await setLocale(lang);
    })();
    await Promise.race([told, timeout]);
  } catch (e) {
    console.warn("set_locale failed; the backend keeps its language", e);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Makes `l` the language of the app: remembers it, then tells the backend (`set_locale`) so that what it answers from now on is in the
 * new language, and only then switches i18next and `<html lang>`. The app reads again what the backend wrote in the old language as soon as
 * the language shown changes (`App`), so the backend must know first. `system` first waits for the system language
 * (`resolveSystemLanguage`, asked once). Resolves when the language is switched: the backend has answered, refused (logged) or kept
 * silent for `SET_LOCALE_TIMEOUT_MS` (logged); none of that keeps the app from starting. A later call that comes while this one waits
 * decides the language: this one then leaves the switch to it.
 */
export async function applyLanguage(l: Language): Promise<void> {
  const mine = ++latestApply;
  try {
    localStorage.setItem(LANGUAGE_KEY, l);
  } catch {
    /* the choice is simply not remembered */
  }
  if (l === "system") await resolveSystemLanguage();
  const effective = effectiveLanguage(l);
  await tellBackend(effective);
  if (mine !== latestApply) return;
  await i18next.changeLanguage(effective);
  setDocumentLanguage(effective);
}

/**
 * The language at the start, before the first render (`main.tsx`): asks the backend for the system language in any case (a later switch to
 * „System“ then finds it at once), applies the stored choice and resolves when it is shown and the backend knows it (or did not answer
 * within `SET_LOCALE_TIMEOUT_MS`). With „System“ that waits for the backend's answer first (at most `SYSTEM_LANGUAGE_TIMEOUT_MS`); a fixed
 * choice does not wait for it.
 */
export async function startLanguage(): Promise<void> {
  void resolveSystemLanguage();
  await applyLanguage(readLanguage());
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
