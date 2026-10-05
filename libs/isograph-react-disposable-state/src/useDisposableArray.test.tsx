import type { ItemCleanupPair } from '@isograph/disposable-types';
import {
  createReferenceCountedPointer,
  type ReferenceCountedPointer,
} from '@isograph/reference-counted-pointer';
import { act, render, screen } from '@testing-library/react';
import React, { startTransition, useLayoutEffect, useState } from 'react';
import { describe, expect, test, vi } from 'vitest';
import {
  type UseDisposableArrayReturn,
  useDisposableArray,
} from './useDisposableArray';

type Entry = ItemCleanupPair<ReferenceCountedPointer<string>>;

function createItem(label: string) {
  const cleanup = vi.fn();
  return {
    cleanup,
    acquire: (): Entry => createReferenceCountedPointer([label, cleanup]),
  };
}

type CommittedRender = UseDisposableArrayReturn<
  ReferenceCountedPointer<string>
>;

// What a caller does: builds the next array from its render's entries by taking a new reference to each kept entry
// and acquiring each added item, then calls setEntries. If a kept entry's reference was already released, the render
// is stale, so it releases everything it acquired and does not call setEntries. Returns whether it called setEntries.
function keepAndAdd(
  committedRender: CommittedRender,
  kept: ReadonlyArray<Entry>,
  added: ReadonlyArray<ReturnType<typeof createItem>>,
): boolean {
  const addedEntries = added.map((item) => item.acquire());
  const clones: Entry[] = [];
  for (const entry of kept) {
    const clone = entry[0].cloneIfNotDisposed();
    if (clone == null) {
      for (const acquired of [...clones, ...addedEntries]) {
        acquired[1]();
      }
      return false;
    }
    clones.push(clone);
  }
  committedRender.setEntries([...clones, ...addedEntries]);
  return true;
}

function labelsOf(entries: ReadonlyArray<Entry>) {
  return entries.map((entry) => entry[0].getItemIfNotDisposed()).join(',');
}

// Renders, under StrictMode, a component that holds a useDisposableArray of reference-counted pointers to strings and
// shows their labels. Every commit of a new array records the render's entries and setEntries in `renders`. After the
// mount, it also appends the labels to `commits` and calls `onCommit`. Both run in a layout effect, which runs before
// the passive effect in which useUpdatableDisposableState releases older arrays. Renders are recorded per commit,
// because StrictMode discards the first of the two mount renders.
function renderDisposableArray(onCommit: (labels: string) => void = () => {}) {
  const renders: CommittedRender[] = [];
  const commits: string[] = [];
  const setMountedRef: { current: (mounted: boolean) => void } = {
    current: () => {},
  };

  function Owner() {
    const { entries, setEntries } =
      useDisposableArray<ReferenceCountedPointer<string>>();
    useLayoutEffect(() => {
      renders.push({ entries, setEntries });
      commits.push(labelsOf(entries));
      onCommit(labelsOf(entries));
    }, [entries, setEntries]);
    return <>{`entries: ${labelsOf(entries)}`}</>;
  }

  function Parent() {
    const [mounted, setMounted] = useState(true);
    setMountedRef.current = setMounted;
    return mounted ? <Owner /> : null;
  }

  render(<Parent />, { reactStrictMode: true });
  // StrictMode runs the mount's layout effects twice.
  commits.length = 0;

  return {
    commits,
    latestRender: () => renders[renders.length - 1]!,
    setMounted: (mounted: boolean) => setMountedRef.current(mounted),
  };
}

describe('useDisposableArray', () => {
  test('dropping the first of three entries and adding a fourth releases the first after the new array commits', () => {
    const a = createItem('a');
    const b = createItem('b');
    const c = createItem('c');
    const d = createItem('d');
    let aCleanupCallsAtCommit: number | null = null;
    const owner = renderDisposableArray((labels) => {
      if (labels === 'b,c,d') {
        aCleanupCallsAtCommit = a.cleanup.mock.calls.length;
      }
    });

    act(() => {
      keepAndAdd(owner.latestRender(), [], [a, b, c]);
    });
    expect(screen.getByText('entries: a,b,c')).toBeTruthy();

    act(() => {
      const committedRender = owner.latestRender();
      keepAndAdd(committedRender, committedRender.entries.slice(-2), [d]);
    });
    expect(screen.getByText('entries: b,c,d')).toBeTruthy();
    expect(aCleanupCallsAtCommit).toBe(0);
    expect(a.cleanup).toHaveBeenCalledOnce();
    expect(b.cleanup).not.toHaveBeenCalled();
    expect(c.cleanup).not.toHaveBeenCalled();
    expect(d.cleanup).not.toHaveBeenCalled();

    act(() => owner.setMounted(false));
    for (const item of [a, b, c, d]) {
      expect(item.cleanup).toHaveBeenCalledOnce();
    }
  });

  test('of two calls before a commit, the second is rendered and the first is released once', () => {
    const a = createItem('a');
    const x = createItem('x');
    const y = createItem('y');
    const owner = renderDisposableArray();
    act(() => {
      keepAndAdd(owner.latestRender(), [], [a]);
    });

    act(() => {
      const committedRender = owner.latestRender();
      keepAndAdd(committedRender, committedRender.entries, [x]);
      keepAndAdd(committedRender, committedRender.entries, [y]);
    });

    expect(screen.getByText('entries: a,y')).toBeTruthy();
    expect(owner.commits).not.toContain('a,x');
    expect(x.cleanup).toHaveBeenCalledOnce();
    expect(a.cleanup).not.toHaveBeenCalled();
    expect(y.cleanup).not.toHaveBeenCalled();

    act(() => owner.setMounted(false));
    for (const item of [a, x, y]) {
      expect(item.cleanup).toHaveBeenCalledOnce();
    }
  });

  test('a caller from an older committed render finds its clones fail and stores nothing', () => {
    const a = createItem('a');
    const b = createItem('b');
    const c = createItem('c');
    const owner = renderDisposableArray();
    act(() => {
      keepAndAdd(owner.latestRender(), [], [a]);
    });
    const olderRender = owner.latestRender();
    act(() => {
      keepAndAdd(olderRender, olderRender.entries, [b]);
    });
    expect(screen.getByText('entries: a,b')).toBeTruthy();

    // The older render's array was released after 'a,b' committed. Item a is still held by the newer array, but the
    // older render's reference to it is released, so cloning it fails.
    let calledSetEntries: boolean | null = null;
    act(() => {
      calledSetEntries = keepAndAdd(olderRender, olderRender.entries, [c]);
    });

    expect(calledSetEntries).toBe(false);
    expect(owner.commits).toEqual(['a', 'a,b']);
    expect(screen.getByText('entries: a,b')).toBeTruthy();
    expect(c.cleanup).toHaveBeenCalledOnce();
    expect(a.cleanup).not.toHaveBeenCalled();
    expect(b.cleanup).not.toHaveBeenCalled();

    act(() => owner.setMounted(false));
    for (const item of [a, b, c]) {
      expect(item.cleanup).toHaveBeenCalledOnce();
    }
  });

  test('an urgent call and a transition call from the same committed array commit in turn and release every item once', () => {
    const a = createItem('a');
    const b = createItem('b');
    const c = createItem('c');
    const d = createItem('d');
    const owner = renderDisposableArray();
    act(() => {
      keepAndAdd(owner.latestRender(), [], [a, b]);
    });

    act(() => {
      const committedRender = owner.latestRender();
      keepAndAdd(committedRender, committedRender.entries.slice(0, 1), [c]);
      startTransition(() => {
        keepAndAdd(committedRender, committedRender.entries, [d]);
      });
    });

    expect(owner.commits).toEqual(['a,b', 'a,c', 'a,b,d']);
    expect(screen.getByText('entries: a,b,d')).toBeTruthy();
    expect(c.cleanup).toHaveBeenCalledOnce();
    expect(a.cleanup).not.toHaveBeenCalled();
    expect(b.cleanup).not.toHaveBeenCalled();
    expect(d.cleanup).not.toHaveBeenCalled();

    act(() => owner.setMounted(false));
    for (const item of [a, b, c, d]) {
      expect(item.cleanup).toHaveBeenCalledOnce();
    }
  });

  test('unmount releases every entry once, including an array that never committed', () => {
    const a = createItem('a');
    const b = createItem('b');
    const c = createItem('c');
    const e = createItem('e');
    const owner = renderDisposableArray();
    act(() => {
      keepAndAdd(owner.latestRender(), [], [a, b]);
    });

    act(() => {
      const committedRender = owner.latestRender();
      keepAndAdd(committedRender, committedRender.entries, [c]);
      owner.setMounted(false);
    });

    expect(owner.commits).toEqual(['a,b']);
    for (const item of [a, b, c]) {
      expect(item.cleanup).toHaveBeenCalledOnce();
    }

    // After unmount, the last render's entries are released, so the caller cannot keep them. It releases the item it
    // acquired and does not call setEntries.
    let calledSetEntries: boolean | null = null;
    act(() => {
      const committedRender = owner.latestRender();
      calledSetEntries = keepAndAdd(committedRender, committedRender.entries, [
        e,
      ]);
    });
    expect(calledSetEntries).toBe(false);
    expect(e.cleanup).toHaveBeenCalledOnce();
    for (const item of [a, b, c]) {
      expect(item.cleanup).toHaveBeenCalledOnce();
    }
  });
});
