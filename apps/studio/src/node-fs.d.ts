// The few Node APIs the tests use (`ui/cssSource.ts`, `workbench/worldModel.vanilla.test.ts`); the studio has no @types/node.
declare module "node:fs" {
  export function readFileSync(path: URL, encoding: "utf8"): string;
  export function readFileSync(path: string, encoding: "utf8"): string;
  export function existsSync(path: string): boolean;
  export function readdirSync(path: string): string[];
}
