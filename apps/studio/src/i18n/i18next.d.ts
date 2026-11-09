import "i18next";
import type { defaultNS, resources } from "./index";

// Typed keys: a misspelled key is a compile error. English is the reference catalog (`catalogs.test.ts` keeps German equal).
declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: typeof defaultNS;
    resources: (typeof resources)["en"];
  }
}
