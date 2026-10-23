// The one Node API the CSS-source tests use (`ui/cssSource.ts`); the studio has no @types/node.
declare module "node:fs" {
  export function readFileSync(path: URL, encoding: "utf8"): string;
}
