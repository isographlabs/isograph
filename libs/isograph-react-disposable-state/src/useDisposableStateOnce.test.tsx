import type { ItemCleanupPair } from '@isograph/disposable-types';
import { act, render, screen } from '@testing-library/react';
import React, { useLayoutEffect } from 'react';
import { describe, expect, test, vi } from 'vitest';
import {
  type UseDisposableStateOnceReturnValue,
  useDisposableStateOnce,
} from './useDisposableStateOnce';
import { UNASSIGNED_STATE } from './useUpdatableDisposableState';

// A factory whose nth call creates the item `item-n` with its own dispose mock.
function createCountingFactory() {
  const disposes: Array<ReturnType<typeof vi.fn>> = [];
  const factory = vi.fn((): ItemCleanupPair<string> => {
    const dispose = vi.fn();
    disposes.push(dispose);
    return [`item-${disposes.length}`, dispose];
  });
  return { factory, disposes };
}

// Renders the hook's state and records the latest committed return value in `latest`.
function Owner({
  latest,
}: {
  latest: { current: UseDisposableStateOnceReturnValue<string> | null };
}) {
  const result = useDisposableStateOnce<string>();
  useLayoutEffect(() => {
    latest.current = result;
  });
  return (
    <>
      {`state: ${result.state === UNASSIGNED_STATE ? 'unassigned' : result.state}`}
    </>
  );
}

describe('useDisposableStateOnce', () => {
  test('only the first setStateOnce call creates an item, and unmount disposes it', () => {
    const latest: {
      current: UseDisposableStateOnceReturnValue<string> | null;
    } = { current: null };
    const first = createCountingFactory();
    const second = createCountingFactory();
    const { unmount } = render(<Owner latest={latest} />);
    expect(screen.getByText('state: unassigned')).toBeTruthy();

    act(() => {
      latest.current!.setStateOnce(first.factory);
      latest.current!.setStateOnce(first.factory);
      latest.current!.setStateOnce(second.factory);
    });

    expect(screen.getByText('state: item-1')).toBeTruthy();
    expect(first.factory).toHaveBeenCalledOnce();
    expect(second.factory).not.toHaveBeenCalled();
    expect(first.disposes[0]).not.toHaveBeenCalled();

    unmount();

    expect(first.disposes[0]).toHaveBeenCalledOnce();
  });

  test('setStateOnce throws before the initial commit and creates nothing', () => {
    const { factory } = createCountingFactory();
    let error: unknown = null;
    function CallsDuringRender() {
      const { setStateOnce } = useDisposableStateOnce<string>();
      try {
        setStateOnce(factory);
      } catch (thrown) {
        error = thrown;
      }
      return null;
    }

    render(<CallsDuringRender />);

    expect(error).toBeInstanceOf(Error);
    expect(factory).not.toHaveBeenCalled();
  });

  // StrictMode's simulated unmount and remount run the component's layout and passive effect cleanups and setups
  // without a render, which is what an Activity does when it hides and then shows the component. React 18, which this
  // package tests with, has no Activity.
  test("StrictMode's simulated unmount and remount neither dispose nor recreate the item", () => {
    const { factory, disposes } = createCountingFactory();
    // A child's layout effect runs during the first commit, after the parent's insertion effects.
    function Child({
      setStateOnce,
    }: {
      setStateOnce: (f: typeof factory) => void;
    }) {
      useLayoutEffect(() => {
        setStateOnce(factory);
      }, [setStateOnce]);
      return null;
    }
    function Parent() {
      const { state, setStateOnce } = useDisposableStateOnce<string>();
      return (
        <>
          {`state: ${state === UNASSIGNED_STATE ? 'unassigned' : state}`}
          <Child setStateOnce={setStateOnce} />
        </>
      );
    }

    const { unmount } = render(<Parent />, { reactStrictMode: true });

    // The child's layout effect runs again on the simulated remount and calls setStateOnce again, which does nothing.
    expect(factory).toHaveBeenCalledOnce();
    expect(disposes[0]).not.toHaveBeenCalled();
    expect(screen.getByText('state: item-1')).toBeTruthy();

    unmount();

    expect(disposes[0]).toHaveBeenCalledOnce();
  });

  test('a setStateOnce call after unmount creates nothing', () => {
    const latest: {
      current: UseDisposableStateOnceReturnValue<string> | null;
    } = { current: null };
    const { factory } = createCountingFactory();
    const { unmount } = render(<Owner latest={latest} />);
    unmount();

    latest.current!.setStateOnce(factory);

    expect(factory).not.toHaveBeenCalled();
  });
});
