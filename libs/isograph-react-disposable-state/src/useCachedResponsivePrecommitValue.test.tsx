import { describe, test, vi, expect, assert } from 'vitest';
import { ParentCache } from './ParentCache';
import { ItemCleanupPair } from '@isograph/disposable-types';
import { useCachedResponsivePrecommitValue } from './useCachedResponsivePrecommitValue';
import React from 'react';
import { create } from 'react-test-renderer';
import { CacheItem, CacheItemState } from './CacheItem';

function getItem<T>(cache: ParentCache<T>): CacheItem<T> | null {
  return (cache as any).__cacheItem;
}

function getState<T>(cacheItem: CacheItem<T>): CacheItemState<T> {
  return (cacheItem as any).__state;
}

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

describe('useCachedResponsivePrecommitValue', () => {
  test('on initial render, it should call getOrPopulateAndTemporaryRetain', async () => {
    const disposeItem = vi.fn();
    const factory = vi.fn(() => {
      const pair: ItemCleanupPair<number> = [1, disposeItem];
      return pair;
    });
    const cache = new ParentCache(factory);
    const getOrPopulateAndTemporaryRetain = vi.spyOn(
      cache,
      'getOrPopulateAndTemporaryRetain',
    );

    const componentCommits = vi.fn();
    const hookOnCommit = vi.fn();
    const render = vi.fn();
    function TestComponent() {
      render();
      React.useEffect(componentCommits);

      const data = useCachedResponsivePrecommitValue(cache, hookOnCommit);

      expect(render).toBeCalledTimes(1);
      expect(componentCommits).not.toBeCalled();
      expect(hookOnCommit).not.toBeCalled();
      expect(data).toEqual({ state: 1 });
      expect(factory).toBeCalledTimes(1);
      expect(disposeItem).not.toBeCalled();
      expect(getOrPopulateAndTemporaryRetain).toBeCalledTimes(1);

      // TODO we should assert that permanentRetainIfNotDisposed was called
      // on the cache item.

      return <div />;
    }

    await awaitableCreate(<TestComponent />, false);

    expect(componentCommits).toBeCalledTimes(1);
    expect(hookOnCommit).toBeCalledTimes(1);
    expect(render).toHaveBeenCalledTimes(1);
  });

  test('on commit, it should call the provided callback and empty the parent cache', async () => {
    const disposeItem = vi.fn();
    const factory = vi.fn(() => {
      const pair: ItemCleanupPair<number> = [1, disposeItem];
      return pair;
    });
    const cache = new ParentCache(factory);

    const componentCommits = vi.fn();
    const hookOnCommit = vi.fn();
    const render = vi.fn();
    function TestComponent() {
      render();
      expect(render).toHaveBeenCalledTimes(1);
      const data = useCachedResponsivePrecommitValue(cache, hookOnCommit);

      React.useEffect(() => {
        componentCommits();
        expect(componentCommits).toHaveBeenCalledTimes(1);
        expect(hookOnCommit).toBeCalledTimes(1);
        expect(hookOnCommit.mock.calls[0][0][0]).toBe(1);
        expect(typeof hookOnCommit.mock.calls[0][0][1]).toBe('function');
        expect(factory).toBeCalledTimes(1);
        expect(disposeItem).not.toBeCalled();
        expect(cache.isEmpty()).toBe(true);
      }, []);

      expect(factory).toBeCalledTimes(1);
      expect(disposeItem).not.toBeCalled();
      return <div />;
    }

    await awaitableCreate(<TestComponent />, false);
    expect(componentCommits).toBeCalledTimes(1);
    expect(render).toHaveBeenCalledTimes(1);
  });

  test('after commit, on subsequent renders it should return null', async () => {
    const disposeItem = vi.fn();
    const factory = vi.fn(() => {
      const pair: ItemCleanupPair<number> = [1, disposeItem];
      return pair;
    });
    const cache = new ParentCache(factory);

    const componentCommits = vi.fn();
    const hookOnCommit = vi.fn();
    let setState;
    let initialRender = true;
    function TestComponent() {
      const [, _setState] = React.useState(null);
      setState = _setState;
      const value = useCachedResponsivePrecommitValue(cache, hookOnCommit);

      if (initialRender && value !== null) {
        initialRender = false;
        expect(value).toEqual({ state: 1 });
      } else {
        expect(value).toEqual(null);
      }

      React.useEffect(() => {
        componentCommits();
        expect(componentCommits).toHaveBeenCalledTimes(1);
        expect(hookOnCommit).toBeCalledTimes(1);
        expect(factory).toBeCalledTimes(1);
        expect(disposeItem).not.toBeCalled();
      }, []);

      return <div />;
    }

    await awaitableCreate(<TestComponent />, false);

    expect(componentCommits).toHaveBeenCalledTimes(1);

    // Trigger a re-render
    setState({});
    await shortPromise();
    expect(initialRender).toBe(false);
  });

  test(
    'on repeated pre-commit renders, if the temporary retain is not disposed, ' +
      'it should re-call getOrPopulateAndTemporaryRetain but not call factory again',
    async () => {
      const disposeItem = vi.fn();
      const factory = vi.fn(() => {
        const pair: ItemCleanupPair<number> = [1, disposeItem];
        return pair;
      });
      const cache = new ParentCache(factory);
      const getOrPopulateAndTemporaryRetain = vi.spyOn(
        cache,
        'getOrPopulateAndTemporaryRetain',
      );

      const componentCommits = vi.fn();
      const hookOnCommit = vi.fn();
      const render = vi.fn();
      let renderCount = 0;
      function TestComponent() {
        render();
        const value = useCachedResponsivePrecommitValue(cache, hookOnCommit);

        expect(value).toEqual({ state: 1 });
        expect(factory).toHaveBeenCalledTimes(1);

        renderCount++;
        expect(getOrPopulateAndTemporaryRetain).toHaveBeenCalledTimes(
          renderCount,
        );

        React.useEffect(() => {
          componentCommits();
          expect(componentCommits).toHaveBeenCalledTimes(1);
          expect(hookOnCommit).toBeCalledTimes(1);
          expect(factory).toBeCalledTimes(1);
          expect(disposeItem).not.toBeCalled();
        }, []);

        return <div />;
      }

      const { promise, isResolvedRef, resolve } = promiseAndResolver();
      await awaitableCreate(
        <React.Suspense fallback={<div />}>
          <TestComponent />
          <Suspender promise={promise} isResolvedRef={isResolvedRef} />
        </React.Suspense>,
        true,
      );

      expect(componentCommits).toHaveBeenCalledTimes(0);
      expect(render).toHaveBeenCalledTimes(1);

      resolve();
      await shortPromise();

      expect(componentCommits).toHaveBeenCalledTimes(1);
      expect(render).toHaveBeenCalledTimes(2);
    },
  );

  test(
    'on repeated pre-commit renders, if the temporary retain is disposed, ' +
      'it should re-call getOrPopulateAndTemporaryRetain and factory',
    async () => {
      const disposeItem = vi.fn();
      let factoryValue = 0;
      const factory = vi.fn(() => {
        factoryValue++;
        const pair: ItemCleanupPair<number> = [factoryValue, disposeItem];
        return pair;
      });
      const cache = new ParentCache(factory);

      const getOrPopulateAndTemporaryRetain = vi.spyOn(
        cache,
        'getOrPopulateAndTemporaryRetain',
      );

      const componentCommits = vi.fn();
      const hookOnCommit = vi.fn();
      const render = vi.fn();
      function TestComponent() {
        render();
        const value = useCachedResponsivePrecommitValue(cache, hookOnCommit);

        expect(value).toEqual({ state: factoryValue });

        React.useEffect(() => {
          componentCommits();
          expect(cache.isEmpty()).toBe(true);
          expect(componentCommits).toHaveBeenCalledTimes(1);
          expect(hookOnCommit).toBeCalledTimes(1);
          expect(hookOnCommit.mock.calls[0][0][0]).toBe(2);
          expect(factory).toBeCalledTimes(2);
          expect(disposeItem).toBeCalledTimes(1);
        }, []);

        if (render.mock.calls.length === 1) {
          expect(factory).toHaveBeenCalledTimes(1);
          // First render, dispose the temporary retain
          expect(disposeItem).toBeCalledTimes(0);
          getOrPopulateAndTemporaryRetain.mock.results[0].value[2]();
          expect(disposeItem).toBeCalledTimes(1);
          expect(getOrPopulateAndTemporaryRetain).toHaveBeenCalledTimes(1);
        } else {
          expect(factory).toHaveBeenCalledTimes(2);
          expect(getOrPopulateAndTemporaryRetain).toHaveBeenCalledTimes(2);
        }

        return <div />;
      }

      const { promise, isResolvedRef, resolve } = promiseAndResolver();
      await awaitableCreate(
        <React.Suspense fallback={<div />}>
          <TestComponent />
          <Suspender promise={promise} isResolvedRef={isResolvedRef} />
        </React.Suspense>,
        true,
      );

      expect(componentCommits).toHaveBeenCalledTimes(0);
      expect(render).toHaveBeenCalledTimes(1);

      resolve();
      await shortPromise();

      expect(componentCommits).toHaveBeenCalledTimes(1);
      expect(render).toHaveBeenCalledTimes(2);
    },
  );

  test(
    'if the item has been disposed between the render and the commit, ' +
      'and the parent cache is empty, it will call factory again, re-render an ' +
      'additional time and called onCommit with the newly generated item',
    async () => {
      const disposeItem = vi.fn();
      let factoryCount = 0;
      const factory = vi.fn(() => {
        factoryCount++;
        const pair: ItemCleanupPair<number> = [factoryCount, disposeItem];
        return pair;
      });
      const cache = new ParentCache(factory);
      const getOrPopulateAndTemporaryRetain = vi.spyOn(
        cache,
        'getOrPopulateAndTemporaryRetain',
      );
      const getAndPermanentRetainIfPresent = vi.spyOn(
        cache,
        'getAndPermanentRetainIfPresent',
      );

      const componentCommits = vi.fn();
      const hookOnCommit = vi.fn();
      const render = vi.fn();
      function TestComponent() {
        render();

        useCachedResponsivePrecommitValue(cache, hookOnCommit);

        React.useEffect(() => {
          componentCommits();
          expect(getOrPopulateAndTemporaryRetain).toHaveBeenCalledTimes(1);
          expect(getAndPermanentRetainIfPresent).toHaveBeenCalledTimes(1);
          expect(getAndPermanentRetainIfPresent.mock.results[0].value).toBe(
            null,
          );
          expect(factory).toHaveBeenCalledTimes(2);
          expect(cache.isEmpty()).toBe(true);
          expect(hookOnCommit).toHaveBeenCalledTimes(1);
          expect(hookOnCommit.mock.calls[0][0][0]).toBe(2);
        }, []);

        return <div />;
      }

      // wat is going on?
      //
      // We want to test a scenario where the item is disposed between the render and
      // the commit.
      //
      // The subcomponents are rendered in order: TestComponent followed by CodeExecutor.
      //
      // - During TestComponent's render, it will populate the cache.
      // - Then, CodeExecutor will render, and dispose the temporary retain,
      //   disposing the cache item. The parent cache will be empty as well.
      // - Then, TestComponent commits.
      let initialRender = true;
      function CodeExecutor() {
        if (initialRender) {
          // This code executes after the initial render of TestComponent, but before
          // it commits.
          expect(disposeItem).not.toHaveBeenCalled();
          getOrPopulateAndTemporaryRetain.mock.results[0].value[2]();
          expect(disposeItem).toHaveBeenCalledTimes(1);
          expect(cache.isEmpty()).toBe(true);

          expect(render).toHaveBeenCalledTimes(1);
          expect(hookOnCommit).toBeCalledTimes(0);
          expect(componentCommits).toBeCalledTimes(0);
          expect(factory).toHaveBeenCalledTimes(1);

          initialRender = false;
        }

        return null;
      }

      const element = await awaitableCreate(
        <>
          <TestComponent />
          <CodeExecutor />
        </>,
        false,
      );

      // This code executes after the commit and re-render of TestComponent.
      // The commit triggers a re-render, because the item was disposed.
      expect(render).toHaveBeenCalledTimes(2);
      expect(factory).toBeCalledTimes(2);
    },
  );

  test(
    'if, between the render and the commit, the item has been disposed, ' +
      'and the parent cache is not empty, it will not call factory again, will re-render ' +
      'an additional time and will call onCommit with the value in the parent cache',
    async () => {
      const disposeItem = vi.fn();
      let factoryCount = 0;
      const factory = vi.fn(() => {
        factoryCount++;
        const pair: ItemCleanupPair<number> = [factoryCount, disposeItem];
        return pair;
      });
      const cache = new ParentCache(factory);
      const getAndPermanentRetainIfPresent = vi.spyOn(
        cache,
        'getAndPermanentRetainIfPresent',
      );

      const componentCommits = vi.fn();
      const hookOnCommit = vi.fn();
      const render = vi.fn();
      function TestComponent() {
        render();
        useCachedResponsivePrecommitValue(cache, hookOnCommit);

        React.useEffect(() => {
          componentCommits();
          // Note that we called getOrPopulateAndTemporaryRetain during CodeExecutor, hence 2
          expect(getOrPopulateAndTemporaryRetain).toHaveBeenCalledTimes(2);
          expect(getAndPermanentRetainIfPresent).toHaveBeenCalledTimes(1);
          expect(getAndPermanentRetainIfPresent.mock.results[0].value[0]).toBe(
            2,
          );
          expect(factory).toHaveBeenCalledTimes(2);
          expect(hookOnCommit).toHaveBeenCalledTimes(1);
          expect(hookOnCommit.mock.calls[0][0][0]).toBe(2);
        }, []);

        return <div />;
      }

      const getOrPopulateAndTemporaryRetain = vi.spyOn(
        cache,
        'getOrPopulateAndTemporaryRetain',
      );

      // wat is going on?
      //
      // We want to test a scenario where the item is disposed between the render and
      // the commit.
      //
      // The subcomponents are rendered in order: TestComponent followed by CodeExecutor.
      //
      // - During TestComponent's render, it will populate the cache.
      // - Then, CodeExecutor will render, and dispose the temporary retain,
      //   disposing the cache item. It will then repopulate the parent cache.
      // - Then, TestComponent commits.
      let initialRender = true;
      function CodeExecutor() {
        if (initialRender) {
          // This code executes after the initial render of TestComponent, but before
          // it commits.
          expect(disposeItem).not.toHaveBeenCalled();
          getOrPopulateAndTemporaryRetain.mock.results[0].value[2]();
          expect(disposeItem).toHaveBeenCalledTimes(1);
          expect(cache.isEmpty()).toBe(true);

          cache.getOrPopulateAndTemporaryRetain();
          expect(cache.isEmpty()).toBe(false);
          // The factory function was called when we called getOrPopulateAndTemporaryRetain
          expect(factory).toHaveBeenCalledTimes(2);

          expect(render).toHaveBeenCalledTimes(1);
          expect(hookOnCommit).toBeCalledTimes(0);
          expect(componentCommits).toBeCalledTimes(0);

          initialRender = false;
        }

        return null;
      }

      const element = await awaitableCreate(
        <React.Suspense fallback="fallback">
          <TestComponent />
          <CodeExecutor />
        </React.Suspense>,
        false,
      );

      // This code executes after the commit and re-render of TestComponent.
      // The commit triggers a re-render, because the item was disposed.
      expect(render).toHaveBeenCalledTimes(2);
      // Note that this is the same number of calls as inside of CodeExecutor,
      // implying that the factory function was not called again.
      expect(factory).toBeCalledTimes(2);
    },
  );

  test(
    'After render but before commit, the item will ' +
      'be in the parent cache, temporarily retained',
    async () => {
      const disposeItem = vi.fn();
      const factory = vi.fn(() => {
        const pair: ItemCleanupPair<number> = [1, disposeItem];
        return pair;
      });
      const cache = new ParentCache(factory);

      const componentCommits = vi.fn();
      const hookOnCommit = vi.fn();
      const render = vi.fn();
      function TestComponent() {
        render();

        useCachedResponsivePrecommitValue(cache, hookOnCommit);

        React.useEffect(() => {
          componentCommits();
        }, []);

        return <div />;
      }

      // wat is going on?
      //
      // We want to test a scenario where the component unmounts before committing.
      // However, we cannot distinguish between an unmount before commit and a
      // render and a commit that hasn't happened yet.
      //
      // This can be simulated with suspense.
      //
      // This test and 'on initial render, it should call getOrPopulateAndTemporaryRetain'
      // can be merged

      const { promise, isResolvedRef } = promiseAndResolver();
      const element = await awaitableCreate(
        <React.Suspense fallback={null}>
          <TestComponent />
          <Suspender promise={promise} isResolvedRef={isResolvedRef} />
        </React.Suspense>,
        true,
      );

      // This code executes after the commit and re-render of TestComponent.
      // The commit triggers a re-render, because the item was disposed.
      expect(render).toHaveBeenCalledTimes(1);
      expect(componentCommits).toHaveBeenCalledTimes(0);
      const item = getItem(cache)!;
      const state = getState(item);
      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.permanentRetainCount).toBe(0);
      expect(state.temporaryRetainCount).toBe(1);
    },
  );
});
