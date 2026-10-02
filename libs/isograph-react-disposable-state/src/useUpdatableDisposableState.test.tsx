import type { ItemCleanupPair } from '@isograph/disposable-types';
import { act, render, screen } from '@testing-library/react';
import React, {
  type MutableRefObject,
  StrictMode,
  useEffect,
  useLayoutEffect,
} from 'react';
import { create } from 'react-test-renderer';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { ParentCache } from './ParentCache';
import { useDisposableState } from './useDisposableState';
import { useUpdatableDisposableClearableState } from './useUpdatableDisposableClearableState';
import {
  type DisposableSetStateAction,
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from './useUpdatableDisposableState';

function Suspender({
  promise,
  isResolvedRef,
}: {
  isResolvedRef: MutableRefObject<boolean>;
  promise: Promise<unknown>;
}) {
  if (!isResolvedRef.current) {
    throw promise;
  }
  return null;
}

function shortPromise() {
  let resolve;
  const promise = new Promise((_resolve) => {
    resolve = _resolve;
  });

  setTimeout(resolve, 1);
  return promise;
}

function promiseAndResolver() {
  let resolve;
  const isResolvedRef = {
    current: false,
  };
  const promise = new Promise((r) => {
    resolve = r;
  });
  return {
    promise,
    resolve: () => {
      isResolvedRef.current = true;
      resolve();
    },
    isResolvedRef,
  };
}

// The fact that sometimes we need to render in concurrent mode and sometimes
// not is a bit worrisome.
async function awaitableCreate(Component, isConcurrent: boolean) {
  const element = create(
    <StrictMode>{Component}</StrictMode>,

    isConcurrent ? { unstable_isConcurrent: true } : undefined,
  );
  await shortPromise();
  return element;
}

describe('useUpdatableDisposableState during its first commit', () => {
  // A child's effects run before its parent's effects of the same kind, and every layout effect runs before any
  // passive effect, so a child effect runs after the parent's DOM is committed and before the parent's own effects.
  // DOM event handlers of the parent can also run in that window.
  test.each([
    ['useLayoutEffect', useLayoutEffect],
    ['useEffect', useEffect],
  ])(
    'setState called from a child %s puts the item in state, and the item is disposed on unmount',
    (_effectName, useChildEffect) => {
      const dispose = vi.fn();
      let setStateError: unknown = null;
      function Child({
        setState,
      }: {
        setState: (pair: ItemCleanupPair<number>) => void;
      }) {
        useChildEffect(() => {
          try {
            setState([1, dispose]);
          } catch (error) {
            setStateError = error;
          }
        }, []);
        return null;
      }
      function Owner() {
        const { state, setState } = useUpdatableDisposableState<number>();
        return (
          <>
            {state === UNASSIGNED_STATE ? 'unassigned' : `item ${state}`}
            <Child setState={setState} />
          </>
        );
      }

      const { unmount } = render(<Owner />);

      expect(setStateError).toBeNull();
      expect(screen.getByText('item 1')).toBeTruthy();
      expect(dispose).not.toHaveBeenCalled();
      unmount();
      expect(dispose).toHaveBeenCalledTimes(1);
    },
  );
});

function renderInStrictMode<
  THook extends {
    state: unknown;
    setState: (action: DisposableSetStateAction<number>) => void;
  },
>(useHook: () => THook) {
  const hookRef: { current: THook | null } = { current: null };
  function Owner() {
    const hook = useHook();
    hookRef.current = hook;
    return (
      <>
        {hook.state === UNASSIGNED_STATE
          ? 'unassigned'
          : `item ${String(hook.state)}`}
      </>
    );
  }
  const { unmount } = render(
    <StrictMode>
      <Owner />
    </StrictMode>,
  );
  function setState(action: DisposableSetStateAction<number>) {
    act(() => {
      hookRef.current!.setState(action);
    });
  }
  return { hookRef, setState, unmount };
}

describe('useUpdatableDisposableState setState with an updater', () => {
  test('the updater receives the latest queued item, so a stale settle is ignored', () => {
    const [pending1, settled1, pending2, settled2] = [1, 10, 2, 20];
    const disposePending1 = vi.fn();
    const disposeSettled1 = vi.fn();
    const disposePending2 = vi.fn();
    const disposeSettled2 = vi.fn();
    const { hookRef, setState, unmount } = renderInStrictMode(() =>
      useUpdatableDisposableState<number>(),
    );

    setState([pending1, disposePending1]);
    expect(screen.getByText('item 1')).toBeTruthy();

    act(() => {
      const { setState } = hookRef.current!;
      setState([pending2, disposePending2]);
      setState((current) =>
        current === pending2 ? [settled2, disposeSettled2] : current,
      );
      // Request 1 settles after request 2 replaced it. The rendered item is
      // still pending1, but the updater receives settled2.
      setState((current) =>
        current === pending1 ? [settled1, disposeSettled1] : current,
      );
    });

    expect(screen.getByText('item 20')).toBeTruthy();
    expect(disposePending1).toHaveBeenCalledTimes(1);
    expect(disposePending2).toHaveBeenCalledTimes(1);
    expect(disposeSettled2).not.toHaveBeenCalled();

    unmount();
    expect(disposeSettled2).toHaveBeenCalledTimes(1);
    // The hook never took ownership of request 1's settled pair.
    expect(disposeSettled1).not.toHaveBeenCalled();
  });

  test('a pair returned from the updater replaces the current pair, which is disposed after commit', () => {
    const dispose1 = vi.fn();
    const dispose2 = vi.fn();
    const { hookRef, setState, unmount } = renderInStrictMode(() =>
      useUpdatableDisposableState<number>(),
    );
    setState([1, dispose1]);

    act(() => {
      hookRef.current!.setState((current) =>
        current === UNASSIGNED_STATE ? current : [current + 1, dispose2],
      );
      expect(dispose1).not.toHaveBeenCalled();
    });

    expect(screen.getByText('item 2')).toBeTruthy();
    expect(dispose1).toHaveBeenCalledTimes(1);
    expect(dispose2).not.toHaveBeenCalled();

    unmount();
    expect(dispose1).toHaveBeenCalledTimes(1);
    expect(dispose2).toHaveBeenCalledTimes(1);
  });

  test('returning current from the updater keeps the item and disposes nothing', () => {
    const dispose1 = vi.fn();
    const { setState, unmount } = renderInStrictMode(() =>
      useUpdatableDisposableState<number>(),
    );

    // With nothing in state, the updater receives UNASSIGNED_STATE.
    setState((current) => current);
    expect(screen.getByText('unassigned')).toBeTruthy();

    setState([1, dispose1]);
    setState((current) => current);
    setState((current) => current);

    expect(screen.getByText('item 1')).toBeTruthy();
    expect(dispose1).not.toHaveBeenCalled();

    unmount();
    expect(dispose1).toHaveBeenCalledTimes(1);
  });

  test('under StrictMode, every pair the hook took ownership of is disposed exactly once', () => {
    const dispose0 = vi.fn();
    const dispose1 = vi.fn();
    const dispose2 = vi.fn();
    const { hookRef, setState, unmount } = renderInStrictMode(() =>
      useUpdatableDisposableState<number>(),
    );

    setState([0, dispose0]);
    act(() => {
      const { setState } = hookRef.current!;
      setState((current) => (current === 0 ? [1, dispose1] : current));
      setState((current) => (current === 1 ? [2, dispose2] : current));
      setState((current) => (current === 1 ? [3, vi.fn()] : current));
    });

    expect(screen.getByText('item 2')).toBeTruthy();
    expect(dispose0).toHaveBeenCalledTimes(1);
    expect(dispose1).toHaveBeenCalledTimes(1);
    expect(dispose2).not.toHaveBeenCalled();

    unmount();
    expect(dispose0).toHaveBeenCalledTimes(1);
    expect(dispose1).toHaveBeenCalledTimes(1);
    expect(dispose2).toHaveBeenCalledTimes(1);
  });

  test('useUpdatableDisposableClearableState passes UNASSIGNED_STATE to the updater after clearState', () => {
    const dispose1 = vi.fn();
    const dispose2 = vi.fn();
    const { hookRef, setState, unmount } = renderInStrictMode(() =>
      useUpdatableDisposableClearableState<number>(),
    );
    setState([1, dispose1]);
    act(() => {
      hookRef.current!.clearState();
    });
    expect(dispose1).toHaveBeenCalledTimes(1);

    const receivedCurrent: unknown[] = [];
    setState((current) => {
      receivedCurrent.push(current);
      return current;
    });
    expect(screen.getByText('unassigned')).toBeTruthy();

    setState((current) =>
      current === UNASSIGNED_STATE ? [2, dispose2] : current,
    );
    expect(screen.getByText('item 2')).toBeTruthy();
    expect(new Set(receivedCurrent)).toEqual(new Set([UNASSIGNED_STATE]));

    unmount();
    expect(dispose1).toHaveBeenCalledTimes(1);
    expect(dispose2).toHaveBeenCalledTimes(1);
  });
});

describe('useDisposableState setState with an updater', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  function createParentCache(item: number) {
    const disposeItem = vi.fn();
    const parentCache = new ParentCache(() => {
      const pair: ItemCleanupPair<number> = [item, disposeItem];
      return pair;
    });
    return { parentCache, disposeItem };
  }

  test.each([
    ['useLayoutEffect', useLayoutEffect],
    ['useEffect', useEffect],
  ])(
    'an updater called from a child %s during the first commit receives the precommit item, and returning it keeps the item',
    (_effectName, useChildEffect) => {
      const { parentCache, disposeItem } = createParentCache(1);
      const receivedCurrent: unknown[] = [];
      function Child({
        setState,
      }: {
        setState: (action: DisposableSetStateAction<number>) => void;
      }) {
        useChildEffect(() => {
          setState((current) => {
            receivedCurrent.push(current);
            return current;
          });
        }, []);
        return null;
      }
      function Owner() {
        const { state, setState } = useDisposableState(parentCache);
        return (
          <>
            {`item ${state}`}
            <Child setState={setState} />
          </>
        );
      }

      const { unmount } = render(
        <StrictMode>
          <Owner />
        </StrictMode>,
      );
      vi.runAllTimers();

      // StrictMode runs the child's effect twice.
      expect(receivedCurrent).toEqual([1, 1]);
      expect(screen.getByText('item 1')).toBeTruthy();
      expect(disposeItem).not.toHaveBeenCalled();

      unmount();
      vi.runAllTimers();
      expect(disposeItem).toHaveBeenCalledTimes(1);
    },
  );

  test('a pair returned from the first updater replaces the precommit item, which is disposed after commit', () => {
    const { parentCache, disposeItem } = createParentCache(1);
    const dispose2 = vi.fn();
    const receivedCurrent: unknown[] = [];
    const { hookRef, unmount } = renderInStrictMode(() =>
      useDisposableState(parentCache),
    );
    // Expire the temporary retain of StrictMode's second render, so that only
    // the hook retains item 1.
    vi.runAllTimers();

    act(() => {
      hookRef.current!.setState((current) => {
        receivedCurrent.push(current);
        return current === 1 ? [2, dispose2] : current;
      });
      expect(disposeItem).not.toHaveBeenCalled();
    });

    expect(receivedCurrent).toEqual([1]);
    expect(screen.getByText('item 2')).toBeTruthy();
    expect(disposeItem).toHaveBeenCalledTimes(1);
    expect(dispose2).not.toHaveBeenCalled();

    unmount();
    vi.runAllTimers();
    expect(disposeItem).toHaveBeenCalledTimes(1);
    expect(dispose2).toHaveBeenCalledTimes(1);
  });

  test('a stale settle updater receives the latest queued item, not the precommit item', () => {
    const [pending1, settled1, pending2, settled2] = [1, 10, 2, 20];
    const { parentCache, disposeItem: disposePending1 } =
      createParentCache(pending1);
    const disposeSettled1 = vi.fn();
    const disposePending2 = vi.fn();
    const disposeSettled2 = vi.fn();
    const receivedBySettle1: unknown[] = [];
    const { hookRef, unmount } = renderInStrictMode(() =>
      useDisposableState(parentCache),
    );
    vi.runAllTimers();
    expect(screen.getByText('item 1')).toBeTruthy();

    act(() => {
      const { setState } = hookRef.current!;
      setState([pending2, disposePending2]);
      setState((current) =>
        current === pending2 ? [settled2, disposeSettled2] : current,
      );
      setState((current) => {
        receivedBySettle1.push(current);
        return current === pending1 ? [settled1, disposeSettled1] : current;
      });
    });

    expect(receivedBySettle1).toEqual([settled2]);
    expect(screen.getByText('item 20')).toBeTruthy();
    expect(disposePending1).toHaveBeenCalledTimes(1);
    expect(disposePending2).toHaveBeenCalledTimes(1);
    expect(disposeSettled2).not.toHaveBeenCalled();

    unmount();
    vi.runAllTimers();
    expect(disposePending1).toHaveBeenCalledTimes(1);
    expect(disposeSettled2).toHaveBeenCalledTimes(1);
    expect(disposeSettled1).not.toHaveBeenCalled();
  });
});

// Temporarily disable unit tests until flakiness is investigated
if (false) {
  describe('useUpdatableDisposableState', () => {
    test('it should return a sentinel value initially and a setter', async () => {
      const render = vi.fn();
      function TestComponent() {
        render();
        const value = useUpdatableDisposableState();
        expect(value.state).toBe(UNASSIGNED_STATE);
        expect(typeof value.setState).toBe('function');
        return null;
      }
      await awaitableCreate(<TestComponent />, false);
      expect(render).toHaveBeenCalledTimes(1);
    });

    test('it should allow you to update the value in state', async () => {
      const render = vi.fn();
      let value;
      function TestComponent() {
        render();
        value = useUpdatableDisposableState();
        return null;
      }
      await awaitableCreate(<TestComponent />, false);
      expect(render).toHaveBeenCalledTimes(1);

      value.setState([1, () => {}]);

      await shortPromise();

      expect(render).toHaveBeenCalledTimes(2);
      expect(value.state).toEqual(1);
    });

    test('it should dispose previous values on commit', async () => {
      const render = vi.fn();
      const componentCommits = vi.fn();
      let value;
      function TestComponent() {
        render();
        value = useUpdatableDisposableState();

        React.useEffect(() => {
          if (value.state === 2) {
            componentCommits();
            expect(disposeInitialState).toHaveBeenCalledTimes(1);
          }
        });
        return null;
      }
      await awaitableCreate(<TestComponent />, false);
      expect(render).toHaveBeenCalledTimes(1);

      const disposeInitialState = vi.fn(() => {});
      value.setState([1, disposeInitialState]);

      await shortPromise();

      expect(render).toHaveBeenCalledTimes(2);
      expect(value.state).toEqual(1);

      value.setState([2, () => {}]);
      expect(disposeInitialState).not.toHaveBeenCalled();

      expect(componentCommits).not.toHaveBeenCalled();
      await shortPromise();
      expect(componentCommits).toHaveBeenCalled();
    });

    test('it should dispose identical previous values on commit', async () => {
      const render = vi.fn();
      const componentCommits = vi.fn();
      let value;
      let hasSetStateASecondTime = false;
      function TestComponent() {
        render();
        value = useUpdatableDisposableState();

        React.useEffect(() => {
          if (hasSetStateASecondTime) {
            componentCommits();
            expect(disposeInitialState).toHaveBeenCalledTimes(1);
          }
        });
        return null;
      }
      await awaitableCreate(<TestComponent />, false);
      expect(render).toHaveBeenCalledTimes(1);

      const disposeInitialState = vi.fn(() => {});
      value.setState([1, disposeInitialState]);

      await shortPromise();

      expect(render).toHaveBeenCalledTimes(2);
      expect(value.state).toEqual(1);

      value.setState([1, () => {}]);
      hasSetStateASecondTime = true;

      expect(disposeInitialState).not.toHaveBeenCalled();

      expect(componentCommits).not.toHaveBeenCalled();
      await shortPromise();
      expect(componentCommits).toHaveBeenCalled();
    });

    test('it should dispose multiple previous values on commit', async () => {
      const render = vi.fn();
      const componentCommits = vi.fn();
      let value;
      let hasSetState = false;
      function TestComponent() {
        render();
        value = useUpdatableDisposableState();

        React.useEffect(() => {
          if (hasSetState) {
            componentCommits();
            expect(dispose1).toHaveBeenCalledTimes(1);
            expect(dispose2).toHaveBeenCalledTimes(1);
          }
        });
        return null;
      }
      // incremental mode => false leads to an immediate (synchronous) commit
      // after the second state update.
      await awaitableCreate(<TestComponent />, true);
      expect(render).toHaveBeenCalledTimes(1);

      const dispose1 = vi.fn(() => {});
      value.setState([1, dispose1]);

      await shortPromise();

      expect(render).toHaveBeenCalledTimes(2);
      expect(value.state).toEqual(1);

      expect(dispose1).not.toHaveBeenCalled();
      const dispose2 = vi.fn(() => {});
      value.setState([2, dispose2]);
      value.setState([2, () => {}]);
      hasSetState = true;

      expect(dispose1).not.toHaveBeenCalled();

      expect(componentCommits).not.toHaveBeenCalled();
      await shortPromise();
      expect(componentCommits).toHaveBeenCalled();
    });

    test('it should throw if setState is called during a render before commit', async () => {
      let didCatch;
      function TestComponent() {
        const value = useUpdatableDisposableState<number>();
        try {
          value.setState([0, () => {}]);
        } catch {
          didCatch = true;
        }
        return null;
      }

      await awaitableCreate(<TestComponent />, false);

      expect(didCatch).toBe(true);
    });

    test('it should not throw if setState is called during render after commit', async () => {
      let value;
      const cleanupFn = vi.fn();
      const sawCorrectValue = vi.fn();
      let shouldSetHookState = false;
      let setState;
      function TestComponent() {
        value = useUpdatableDisposableState<number>();
        const [, _setState] = React.useState();
        setState = _setState;

        if (shouldSetHookState) {
          value.setState([1, cleanupFn]);
          shouldSetHookState = false;
        }

        React.useEffect(() => {
          if (value.state === 1) {
            sawCorrectValue();
          }
        });
        return null;
      }

      await awaitableCreate(<TestComponent />, true);

      shouldSetHookState = true;
      setState({});

      await shortPromise();

      expect(sawCorrectValue).toHaveBeenCalledTimes(1);
      expect(value.state).toBe(1);
    });

    test('it should throw if setState is called after a render before commit', async () => {
      let value;
      const componentCommits = vi.fn();
      function TestComponent() {
        value = useUpdatableDisposableState<number>();
        React.useEffect(() => {
          componentCommits();
        });
        return null;
      }

      const { promise, isResolvedRef, resolve } = promiseAndResolver();
      await awaitableCreate(
        <React.Suspense fallback="fallback">
          <TestComponent />
          <Suspender promise={promise} isResolvedRef={isResolvedRef} />
        </React.Suspense>,
        true,
      );

      expect(componentCommits).not.toHaveBeenCalled();

      expect(() => {
        value.setState([1, () => {}]);
      }).toThrow();
    });

    test(
      'it should dispose items that were set during ' +
        'suspense when the component commits due to unsuspense',
      async () => {
        // Note that "during suspense" implies that there is no commit, so this
        // follows from the descriptions of the previous tests. Nonetheless, we
        // should test this scenario.

        let value;
        const componentCommits = vi.fn();
        const render = vi.fn();
        function TestComponent() {
          render();
          value = useUpdatableDisposableState<number>();
          React.useEffect(() => {
            componentCommits();
          });
          return null;
        }

        let setState;
        function ParentComponent() {
          const [, _setState] = React.useState();
          setState = _setState;
          return (
            <>
              <TestComponent />
              <Suspender promise={promise} isResolvedRef={isResolvedRef} />
            </>
          );
        }

        const { promise, isResolvedRef, resolve } = promiseAndResolver();
        // Do not suspend initially
        isResolvedRef.current = true;
        await awaitableCreate(
          <React.Suspense fallback="fallback">
            <ParentComponent />
          </React.Suspense>,
          true,
        );

        expect(render).toHaveBeenCalledTimes(1);
        expect(componentCommits).toHaveBeenCalledTimes(1);

        // We need to also re-render the suspending component, in this case we do so
        // by triggering a state change on the parent
        isResolvedRef.current = false;
        setState({});

        const cleanup1 = vi.fn();
        value.setState([1, cleanup1]);
        const cleanup2 = vi.fn();
        value.setState([2, cleanup2]);

        await shortPromise();

        // Assert that the state changes were batched due to concurrent mode
        // by noting that only one render occurred.
        expect(render).toHaveBeenCalledTimes(2);
        // Also assert another commit hasn't occurred
        expect(componentCommits).toHaveBeenCalledTimes(1);
        expect(cleanup1).not.toHaveBeenCalled();
        expect(cleanup2).not.toHaveBeenCalled();

        // Now, unsuspend
        isResolvedRef.current = true;
        resolve();
        await shortPromise();

        expect(cleanup1).toHaveBeenCalledTimes(1);
        expect(render).toHaveBeenCalledTimes(3);
        expect(componentCommits).toHaveBeenCalledTimes(2);
      },
    );

    test('it should properly clean up all items passed to setState during suspense on unmount', async () => {
      let value;
      const componentCommits = vi.fn();
      const render = vi.fn();
      function TestComponent() {
        render();
        value = useUpdatableDisposableState<number>();
        React.useEffect(() => {
          componentCommits();
        });
        return null;
      }

      let setState;
      function ParentComponent({
        shouldMountRef,
      }: {
        shouldMountRef: MutableRefObject<boolean>;
      }) {
        const [, _setState] = React.useState();
        setState = _setState;
        return shouldMountRef.current ? (
          <>
            <TestComponent />
            <Suspender promise={promise} isResolvedRef={isResolvedRef} />
          </>
        ) : null;
      }

      const { promise, isResolvedRef } = promiseAndResolver();
      // Do not suspend initially
      isResolvedRef.current = true;
      const shouldMountRef = { current: true };

      await awaitableCreate(
        <React.Suspense fallback="fallback">
          <ParentComponent shouldMountRef={shouldMountRef} />
        </React.Suspense>,
        true,
      );

      expect(render).toHaveBeenCalledTimes(1);
      expect(componentCommits).toHaveBeenCalledTimes(1);

      // We need to also re-render the suspending component, in this case we do so
      // by triggering a state change on the parent
      isResolvedRef.current = false;
      setState({});

      // For thoroughness, we might want to test awaiting a shortPromise() here, so
      // as not to batch these state changes.

      const cleanup1 = vi.fn();
      value.setState([1, cleanup1]);
      const cleanup2 = vi.fn();
      value.setState([2, cleanup2]);

      await shortPromise();

      // Assert that the state changes were batched due to concurrent mode
      // by noting that only one render occurred.
      expect(render).toHaveBeenCalledTimes(2);
      // Also assert another commit hasn't occurred
      expect(componentCommits).toHaveBeenCalledTimes(1);
      expect(cleanup1).not.toHaveBeenCalled();
      expect(cleanup2).not.toHaveBeenCalled();

      // Now, unmount
      shouldMountRef.current = false;
      setState({});

      await shortPromise();

      expect(cleanup1).toHaveBeenCalled();
      expect(cleanup2).toHaveBeenCalled();
    });

    test('it should clean up the item currently in state on unmount', async () => {
      let value;
      const componentCommits = vi.fn();
      const render = vi.fn();
      function TestComponent() {
        render();
        value = useUpdatableDisposableState<number>();
        React.useEffect(() => {
          componentCommits();
        });
        return null;
      }

      let setState;
      function ParentComponent({
        shouldMountRef,
      }: {
        shouldMountRef: MutableRefObject<boolean>;
      }) {
        const [, _setState] = React.useState();
        setState = _setState;
        return shouldMountRef.current ? <TestComponent /> : null;
      }

      const shouldMountRef = { current: true };

      await awaitableCreate(
        <ParentComponent shouldMountRef={shouldMountRef} />,
        true,
      );

      expect(render).toHaveBeenCalledTimes(1);
      expect(componentCommits).toHaveBeenCalledTimes(1);

      const cleanup1 = vi.fn();
      value.setState([1, cleanup1]);

      await shortPromise();
      expect(componentCommits).toHaveBeenCalledTimes(2);
      expect(value.state).toBe(1);

      expect(render).toHaveBeenCalledTimes(2);
      expect(cleanup1).not.toHaveBeenCalled();

      // Now, unmount
      shouldMountRef.current = false;
      setState({});

      await shortPromise();

      expect(cleanup1).toHaveBeenCalled();
      expect(render).toHaveBeenCalledTimes(2);
    });
  });
}
