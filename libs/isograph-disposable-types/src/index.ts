export type CleanupFn = () => void;
export type ItemCleanupPair<T> = [T, CleanupFn];
export type Factory<T> = () => ItemCleanupPair<T>;

export * from './CacheItem';
export * from './ParentCache';
