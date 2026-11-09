// The one backend call of `applyLanguage`, in a module of its own that `index.ts` loads lazily (see there). Behind a dynamic import
// of this file the bundler has nothing to say; a dynamic import of `api.base.ts` itself would draw an INEFFECTIVE_DYNAMIC_IMPORT
// warning, because `api.ts` imports that statically.
export { setLocale } from "../api.base";
