import { init, readLanguage } from "./index";

// Imported first by `main.tsx`. ES modules evaluate in import order, so i18next is initialised before any module of the app's graph
// is evaluated: a `t()` at module scope (a `const TABLE = { label: t("...") }`) then finds the catalog. Before `init` i18next answers
// an empty string, and nothing would tell. (In the tests `vitest.setup.ts` plays this part.)
init(readLanguage());
