export type NonEmptyArray<T> = readonly [T, ...T[]];

export function isNonEmptyArray<T>(arr: readonly T[]): arr is NonEmptyArray<T> {
  return arr.length !== 0;
}
