import "react";

declare module "react" {
  interface CSSProperties {
    /** A custom property set at runtime (`--ds-scale`, `--text-line-fill`): a `style` object names it without a cast. */
    [property: `--${string}`]: string | number | undefined;
  }
}
