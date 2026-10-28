/** Words shared by the app's messages, whatever area says them. */

/** The no-break space between a count and what it counts: a message never breaks between „57“ and „s“. */
export const NBSP = " ";

/** `n` with what it counts, bound by a no-break space: `one` for 1, `many` otherwise. */
export const count = (n: number, one: string, many: string) => `${n}${NBSP}${n === 1 ? one : many}`;
