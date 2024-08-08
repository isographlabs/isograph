import type { CleanupFn, ItemCleanupPair } from '@isograph/disposable-types';

// TODO cloneIfNotDisposed should also return the underlying item

/**
 * Create an undisposed reference-counted pointer guarding a given item.
 *
 * Once all reference-counted pointers guarding a given item have been
 * disposed, the underlying item will be disposed.
 *
 * Additional reference-counted pointers guarding the same item can be
 * created by calling cloneIfNotDisposed().
 *
 * ## Structural sharing
 *
 * Reference counted pointers enable reusing disposable items between
 * application states, so-called structural sharing.
 *
 * If state 1 contains a reference counted pointer to an item, in order
 * to transition to state 2, one would first create an additional
 * reference-counted pointer by calling cloneIfNotDisposed, transition
 * to state 2, then clean up state 1 by disposing of its reference-
 * counted pointers. In this transition, at no time were there zero
 * undisposed reference countend pointers to the disposable item, so it
 * was never disposed, and we could reuse it between states.
 */
export function createReferenceCountedPointer<T>(
  pair: ItemCleanupPair<T>,
): ItemCleanupPair<ReferenceCountedPointer<T>> {
  const originalReferenceCountedPointer = new RefCounter(pair);

  return originalReferenceCountedPointer.retainIfNotDisposed()!;
}

export interface ReferenceCountedPointer<T> {
  isDisposed(): boolean;
  /**
   * Safety: the item returned here is valid for use only as long as the reference
   * counted pointer is not disposed.
   */
  getItemIfNotDisposed(): T | null;
  cloneIfNotDisposed(): ItemCleanupPair<ReferenceCountedPointer<T>> | null;
}

type RefCountState<T> = {
  item: T;
  dispose: CleanupFn;
  // Invariant: >0
  activeReferenceCount: number;
};

// N.B. this could implement ReferenceCountedPointer<T>, but it would not be correct to use it
// as such, since it does not have an associated dispose function that can be called.
//
// Note that there is no way, and should be no way, to determine whether the underlying item
// has been disposed, let alone force it to be disposed! If you need that, you need to keep track
// of all calls to retainIfNotDisposed.
class RefCounter<T> {
  private __state: RefCountState<T> | null;

  /**
   * Private. Do not expose this class directly, as this contructor creates a ReferenceCountedPointer
   * in an invalid state. We must immediately, after creation, call retainIfNotDisposed().
   */
  constructor([item, dispose]: ItemCleanupPair<T>) {
    this.__state = {
      item,
      dispose,
      activeReferenceCount: 0,
    };
  }

  getIfNotDisposed(): T | null {
    return this.__state === null ? null : this.__state.item;
  }

  retainIfNotDisposed(): ItemCleanupPair<ReferenceCountedPointer<T>> | null {
    if (this.__state !== null) {
      this.__state.activeReferenceCount++;

      const activeReference = new ActiveReference(this);

      let disposed = false;
      const dispose = () => {
        if (disposed) {
          throw new Error(
            'Do not dispose an already-disposed ActiveReference.',
          );
        }
        disposed = true;
        if (activeReference.__original === null) {
          throw new Error(
            'Attempted to dispose an active reference, but it was already disposed. ' +
              'This indicates a bug in reference-counted-pointer.',
          );
        }
        activeReference.__original = null;
        if (this.__state === null) {
          throw new Error(
            'Attempted to dispose, but the underlying reference counted pointer was disposed. ' +
              'This indicates a bug in reference-counted-pointer.',
          );
        }
        this.__state.activeReferenceCount--;
        this.__maybeDispose();
      };

      return [activeReference, dispose];
    } else {
      return null;
    }
  }

  private __maybeDispose() {
    if (this.__state === null) {
      throw new Error(
        '__maybeDispose was called, but the reference counted pointer was disposed. ' +
          'This indicates a bug in reference-counted-pointer.',
      );
    }
    if (this.__state.activeReferenceCount === 0) {
      this.__state.dispose();
      this.__state = null;
    }
  }
}

class ActiveReference<T> implements ReferenceCountedPointer<T> {
  // Invariant: __original !== null => the original is not disposed.
  __original: RefCounter<T> | null;

  constructor(original: RefCounter<T>) {
    this.__original = original;
  }

  isDisposed(): boolean {
    return this.__original === null;
  }

  cloneIfNotDisposed(): ItemCleanupPair<ReferenceCountedPointer<T>> | null {
    return this.__original?.retainIfNotDisposed() ?? null;
  }

  getItemIfNotDisposed(): T | null {
    return this.__original?.getIfNotDisposed() ?? null;
  }
}
