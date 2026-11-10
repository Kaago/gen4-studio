/** Words shared by the app's messages, whatever area says them. */

/** The no-break space between a count and what it counts: a message never breaks between „57“ and „s“. */
export const NBSP = " ";

/**
 * The word joiner (U+2060, zero width) around the hyphen of a word that must not break inside, „Basis\u2060-\u2060ROM“: a toast never wraps
 * „Basis-“ / „ROM“. Not the no-break hyphen U+2011: the UI font (Atkinson Hyperlegible Next) has no glyph for it, the hyphen would come from another font.
 */
export const WJ = "\u2060";
