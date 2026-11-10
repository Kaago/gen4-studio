// The backend calls of the language (`set_locale` for `applyLanguage`, `system_language` for `resolveSystemLanguage`), in a module of
// their own that `index.ts` loads lazily (see there). Behind a dynamic import of this file the bundler has nothing to say; a dynamic
// import of `api.base.ts` itself would draw an INEFFECTIVE_DYNAMIC_IMPORT warning, because `api.ts` imports that statically.
export { setLocale, systemLanguage } from "../api.base";
