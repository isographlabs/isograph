export type CleanupFn = () => void;
export type ItemCleanupPair<T> = [T, CleanupFn];
export type Factory<T> = () => ItemCleanupPair<T>;

type ItemsOf<T extends readonly ItemCleanupPair<unknown>[]> = {
  [I in keyof T]: T[I] extends ItemCleanupPair<infer V> ? V : never;
};

export function joinCleanups<
  const T extends readonly ItemCleanupPair<unknown>[],
>(...pairs: T): ItemCleanupPair<ItemsOf<T>> {
  const items = pairs.map(([item]) => item) as ItemsOf<T>;
  const dispose: CleanupFn = () => {
    let error: unknown;
    pairs.reduceRight((_, [, cleanup]) => {
      try {
        cleanup();
      } catch (e) {
        error ??= e;
      }
      return undefined;
    }, undefined);
    if (error !== undefined) {
      throw error;
    }
  };
  return [items, dispose];
}
