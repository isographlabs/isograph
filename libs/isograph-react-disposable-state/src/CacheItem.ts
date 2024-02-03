import {
  CleanupFn,
  Factory,
  ItemCleanupPair,
} from '@isograph/disposable-types';

const DEFAULT_TEMPORARY_RETAIN_TIME = 5000;

export type NotInParentCacheAndDisposed = {
  kind: 'NotInParentCacheAndDisposed';
};
export type NotInParentCacheAndNotDisposed<T> = {
  kind: 'NotInParentCacheAndNotDisposed';
  value: T;
  disposeValue: () => void;

  // Invariant: >0
  permanentRetainCount: number;
};
export type InParentCacheAndNotDisposed<T> = {
  kind: 'InParentCacheAndNotDisposed';
  value: T;
  disposeValue: () => void;
  removeFromParentCache: () => void;

  // Invariant: >0
  temporaryRetainCount: number;

  // Invariant: >= 0
  permanentRetainCount: number;
};

export type CacheItemState<T> =
  | InParentCacheAndNotDisposed<T>
  | NotInParentCacheAndNotDisposed<T>
  | NotInParentCacheAndDisposed;

export type CacheItemOptions = {
  temporaryRetainTime: number;
};

// TODO don't export this class, only export type (interface) instead
// TODO convert cacheitem impl to a getter and setter and free functions

/**
 * CacheItem:
 *
 * Terminology:
 * - TRC = Temporary Retain Count
 * - PRC = Permanent Retain Count
 *
 * A CacheItem<T> can be in three states:
 *   In parent cache? | Item disposed? | TRC | PRC | Name
 *   -----------------+----------------+-----+-----+-------------------------------
 *   In parent cache  | Not disposed   | >0  | >=0 | InParentCacheAndNotDisposed
 *   Removed          | Not disposed   |  0  |  >0 | NotInParentCacheAndNotDisposed
 *   Removed          | Disposed       |  0  |   0 | NotInParentCacheAndNotDisposed
 *
 * A cache item can only move down rows. As in, if its in the parent cache,
 * it can be removed. It can never be replaced in the parent cache. (If a
 * parent cache becomes full again, it will contain a new CacheItem.) The
 * contained item can be disposed, but never un-disposed.
 *
 * So, the valid transitions are:
 * - InParentCacheAndNotDisposed => NotInParentCacheAndNotDisposed
 * - InParentCacheAndNotDisposed => NotInParentCacheAndDisposed
 * - NotInParentCacheAndNotDisposed => NotInParentCacheAndDisposed
 */
export class CacheItem<T> {
  private __state: CacheItemState<T>;
  private __options: CacheItemOptions | null;

  // Private. Do not call this constructor directly. Use
  // createTemporarilyRetainedCacheItem instead. This is because this
  // constructor creates a CacheItem in an invalid state. It must be
  // temporarily retained to enter a valid state, and JavaScript doesn't
  // let you return a tuple from a constructor.
  constructor(
    factory: Factory<T>,
    removeFromParentCache: CleanupFn,
    options: CacheItemOptions | void,
  ) {
    this.__options = options ?? null;
    const [value, disposeValue] = factory();
    this.__state = {
      kind: 'InParentCacheAndNotDisposed',
      value,
      disposeValue,
      removeFromParentCache,
      // NOTE: we are creating the CacheItem in an invalid state. This is okay, because
      // we are immediately calling .temporaryRetain.
      temporaryRetainCount: 0,
      permanentRetainCount: 0,
    };
  }

  getValue(): T {
    switch (this.__state.kind) {
      case 'InParentCacheAndNotDisposed': {
        return this.__state.value;
      }
      case 'NotInParentCacheAndNotDisposed': {
        return this.__state.value;
      }
      default: {
        throw new Error(
          'Attempted to access disposed value from CacheItem. ' +
            'This indicates a bug in react-disposable-state.',
        );
      }
    }
  }

  permanentRetainIfNotDisposed(
    disposeOfTemporaryRetain: CleanupFn,
  ): ItemCleanupPair<T> | null {
    switch (this.__state.kind) {
      case 'InParentCacheAndNotDisposed': {
        let cleared = false;
        this.__state.permanentRetainCount++;
        disposeOfTemporaryRetain();
        return [
          this.__state.value,
          () => {
            if (cleared) {
              throw new Error(
                'A permanent retain should only be cleared once. ' +
                  'This indicates a bug in react-disposable-state.',
              );
            }
            cleared = true;
            switch (this.__state.kind) {
              case 'InParentCacheAndNotDisposed': {
                this.__state.permanentRetainCount--;
                this.__maybeExitInParentCacheAndNotDisposedState(this.__state);
                return;
              }
              case 'NotInParentCacheAndNotDisposed': {
                this.__state.permanentRetainCount--;
                this.__maybeExitNotInParentCacheAndNotDisposedState(
                  this.__state,
                );
                return;
              }
              default: {
                throw new Error(
                  'CacheItem was in a disposed state, but there existed a permanent retain. ' +
                    'This indicates a bug in react-disposable-state.',
                );
              }
            }
          },
        ];
      }
      case 'NotInParentCacheAndNotDisposed': {
        let cleared = false;
        this.__state.permanentRetainCount++;
        disposeOfTemporaryRetain();
        return [
          this.__state.value,
          () => {
            if (cleared) {
              throw new Error(
                'A permanent retain should only be cleared once. ' +
                  'This indicates a bug in react-disposable-state.',
              );
            }
            cleared = true;
            switch (this.__state.kind) {
              case 'NotInParentCacheAndNotDisposed': {
                this.__state.permanentRetainCount--;
                this.__maybeExitNotInParentCacheAndNotDisposedState(
                  this.__state,
                );
                return;
              }
              default: {
                throw new Error(
                  'CacheItem was in an unexpected state. ' +
                    'This indicates a bug in react-disposable-state.',
                );
              }
            }
          },
        ];
      }
      default: {
        // The CacheItem is disposed, so disposeOfTemporaryRetain is a no-op
        return null;
      }
    }
  }

  temporaryRetain(): CleanupFn {
    type TemporaryRetainStatus =
      | 'Uncleared'
      | 'ClearedByCallback'
      | 'ClearedByTimeout';

    switch (this.__state.kind) {
      case 'InParentCacheAndNotDisposed': {
        let status: TemporaryRetainStatus = 'Uncleared';
        this.__state.temporaryRetainCount++;
        const clearTemporaryRetainByCallack: CleanupFn = () => {
          if (status === 'ClearedByCallback') {
            throw new Error(
              'A temporary retain should only be cleared once. ' +
                'This indicates a bug in react-disposable-state.',
            );
          } else if (status === 'Uncleared') {
            switch (this.__state.kind) {
              case 'InParentCacheAndNotDisposed': {
                this.__state.temporaryRetainCount--;
                this.__maybeExitInParentCacheAndNotDisposedState(this.__state);
                clearTimeout(timeoutId);
                return;
              }
              default: {
                throw new Error(
                  'A temporary retain was cleared, for which the CacheItem is in an invalid state. ' +
                    'This indicates a bug in react-disposable-state.',
                );
              }
            }
          }
        };

        const clearTemporaryRetainByTimeout = () => {
          status = 'ClearedByTimeout';
          switch (this.__state.kind) {
            case 'InParentCacheAndNotDisposed': {
              this.__state.temporaryRetainCount--;
              this.__maybeExitInParentCacheAndNotDisposedState(this.__state);
              return;
            }
            default: {
              throw new Error(
                'A temporary retain was cleared, for which the CacheItem is in an invalid state. ' +
                  'This indicates a bug in react-disposable-state.',
              );
            }
          }
        };

        const timeoutId = setTimeout(
          clearTemporaryRetainByTimeout,
          this.__options?.temporaryRetainTime ?? DEFAULT_TEMPORARY_RETAIN_TIME,
        );
        return clearTemporaryRetainByCallack;
      }
      default: {
        throw new Error(
          'temporaryRetain was called, for which the CacheItem is in an invalid state. ' +
            'This indicates a bug in react-disposable-state.',
        );
      }
    }
  }

  permanentRetain(): CleanupFn {
    switch (this.__state.kind) {
      case 'InParentCacheAndNotDisposed': {
        let cleared = false;
        this.__state.permanentRetainCount++;
        return () => {
          if (cleared) {
            throw new Error(
              'A permanent retain should only be cleared once. ' +
                'This indicates a bug in react-disposable-state.',
            );
          }
          cleared = true;
          switch (this.__state.kind) {
            case 'InParentCacheAndNotDisposed': {
              this.__state.permanentRetainCount--;
              this.__maybeExitInParentCacheAndNotDisposedState(this.__state);
              return;
            }
            case 'NotInParentCacheAndNotDisposed': {
              this.__state.permanentRetainCount--;
              this.__maybeExitNotInParentCacheAndNotDisposedState(this.__state);
              return;
            }
            default: {
              throw new Error(
                'CacheItem was in a disposed state, but there existed a permanent retain. ' +
                  'This indicates a bug in react-disposable-state.',
              );
            }
          }
        };
      }
      case 'NotInParentCacheAndNotDisposed': {
        let cleared = false;
        this.__state.permanentRetainCount++;
        return () => {
          if (cleared) {
            throw new Error(
              'A permanent retain should only be cleared once. ' +
                'This indicates a bug in react-disposable-state.',
            );
          }
          cleared = true;
          switch (this.__state.kind) {
            case 'NotInParentCacheAndNotDisposed': {
              this.__state.permanentRetainCount--;
              this.__maybeExitNotInParentCacheAndNotDisposedState(this.__state);
              return;
            }
            default: {
              throw new Error(
                'CacheItem was in an unexpected state. ' +
                  'This indicates a bug in react-disposable-state.',
              );
            }
          }
        };
      }
      default: {
        throw new Error(
          'permanentRetain was called, but the CacheItem is in an invalid state. ' +
            'This indicates a bug in react-disposable-state.',
        );
      }
    }
  }

  private __maybeExitInParentCacheAndNotDisposedState(
    state: InParentCacheAndNotDisposed<T>,
  ) {
    if (state.temporaryRetainCount === 0 && state.permanentRetainCount === 0) {
      state.removeFromParentCache();
      state.disposeValue();
      this.__state = {
        kind: 'NotInParentCacheAndDisposed',
      };
    } else if (state.temporaryRetainCount === 0) {
      state.removeFromParentCache();
      this.__state = {
        kind: 'NotInParentCacheAndNotDisposed',
        value: state.value,
        disposeValue: state.disposeValue,
        permanentRetainCount: state.permanentRetainCount,
      };
    }
  }

  private __maybeExitNotInParentCacheAndNotDisposedState(
    state: NotInParentCacheAndNotDisposed<T>,
  ) {
    if (state.permanentRetainCount === 0) {
      state.disposeValue();
      this.__state = {
        kind: 'NotInParentCacheAndDisposed',
      };
    }
  }
}

export function createTemporarilyRetainedCacheItem<T>(
  factory: Factory<T>,
  removeFromParentCache: CleanupFn,
  options: CacheItemOptions | void,
): [CacheItem<T>, CleanupFn] {
  const cacheItem = new CacheItem(factory, removeFromParentCache, options);
  const disposeTemporaryRetain = cacheItem.temporaryRetain();
  return [cacheItem, disposeTemporaryRetain];
}
