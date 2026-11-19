import type { JsonObject, JsonValue } from './store';

export function isArray(value: unknown): value is readonly unknown[] {
  return Array.isArray(value);
}

/** `object[key]` when `key` is an own property of `object`, so that a key such as
 * `constructor` does not read `Object.prototype`. */
export function ownValue(
  object: JsonObject,
  key: string,
): JsonValue | undefined {
  return Object.hasOwn(object, key) ? object[key] : undefined;
}

/**
 * Creates a copy of the provided value, ensuring any nested objects have their
 * keys sorted such that equivalent values would have identical JSON.stringify
 * results.
 */
export function stableCopy(value: JsonValue): JsonValue {
  if (value == null || typeof value !== 'object') {
    return value;
  }
  if (isArray(value)) {
    return value.map(stableCopy);
  }
  return Object.fromEntries(
    Object.entries(value)
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(([key, item]) => [key, stableCopy(item)]),
  );
}

/** The value under `key`, inserting `create()` first when `map` has none. */
export function getOrInsert<K, V>(map: Map<K, V>, key: K, create: () => V): V {
  const existing = map.get(key);
  if (existing !== undefined) {
    return existing;
  }
  const created = create();
  map.set(key, created);
  return created;
}
