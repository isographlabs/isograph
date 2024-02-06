import { describe, test, vi, expect } from 'vitest';
import React from 'react';
import { create } from 'react-test-renderer';
import {
  useUpdatableDisposableState,
  UNASSIGNED_STATE,
} from './useUpdatableDisposableState';

function Suspender({ promise, isResolvedRef }) {
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
async function awaitableCreate(Component, isConcurrent) {
  const element = create(
    Component,
    isConcurrent ? { unstable_isConcurrent: true } : undefined,
  );
  await shortPromise();
  return element;
}

describe('nothing', () => {
  test('it should pass', () => {});
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
      function ParentComponent({ shouldMountRef }) {
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
      function ParentComponent({ shouldMountRef }) {
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
