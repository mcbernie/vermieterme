/**
 * Serialises a JSON-like value with object keys in sorted order, so two values with the same content give
 * the same string regardless of key order or object identity. Used to compare configurations by content.
 * @returns The canonical JSON string, or `null` for `null`/`undefined`.
 */
export function stableKey(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  return JSON.stringify(sorted(value));
}

/** Copy of `value` whose plain objects have their keys inserted in sorted order (arrays keep their order). */
function sorted(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sorted);
  if (value === null || typeof value !== "object") return value;
  const source = value as Record<string, unknown>;
  const result: Record<string, unknown> = {};
  for (const key of Object.keys(source).sort()) result[key] = sorted(source[key]);
  return result;
}
