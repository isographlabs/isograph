import type { ItemCleanupPair } from '@isograph/disposable-types';
import { describe, expect, test, vi } from 'vitest';
import {
  getPromiseState,
  transpose,
  wrapPromise,
  wrapResolvedValue,
} from '../src/index';

/** A promise and the functions that settle it, which only the test calls. */
function deferred<T>(): {
  readonly promise: Promise<T>;
  readonly resolve: (value: T) => void;
  readonly reject: (error: unknown) => void;
} {
  let resolve: (value: T) => void = () => {};
  let reject: (error: unknown) => void = () => {};
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const item = { name: 'item' };
const message = 'A transposed cleanup should only be called once.';

describe('transpose', () => {
  test('resolve, then cleanup', async () => {
    const pair = deferred<ItemCleanupPair<typeof item>>();
    const innerCleanup = vi.fn();
    const [transposed, cleanup] = transpose(wrapPromise(pair.promise));
    expect(getPromiseState(transposed).kind).toBe('Pending');

    pair.resolve([item, innerCleanup]);
    await expect(transposed.promise).resolves.toBe(item);
    expect(getPromiseState(transposed)).toEqual({ kind: 'Ok', value: item });
    expect(innerCleanup).not.toHaveBeenCalled();

    cleanup();
    expect(innerCleanup).toHaveBeenCalledTimes(1);
  });

  test('cleanup, then resolve', async () => {
    const pair = deferred<ItemCleanupPair<typeof item>>();
    const innerCleanup = vi.fn();
    const [transposed, cleanup] = transpose(wrapPromise(pair.promise));

    cleanup();
    expect(innerCleanup).not.toHaveBeenCalled();

    pair.resolve([item, innerCleanup]);
    await expect(transposed.promise).resolves.toBe(item);
    expect(innerCleanup).toHaveBeenCalledTimes(1);
    expect(getPromiseState(transposed)).toEqual({ kind: 'Ok', value: item });
  });

  test('reject, then cleanup', async () => {
    const pair = deferred<ItemCleanupPair<typeof item>>();
    const error = new Error('rejected');
    const [transposed, cleanup] = transpose(wrapPromise(pair.promise));

    pair.reject(error);
    await expect(transposed.promise).rejects.toBe(error);
    expect(getPromiseState(transposed)).toEqual({ kind: 'Err', error });
    expect(() => cleanup()).not.toThrow();
  });

  test('cleanup, then reject', async () => {
    const pair = deferred<ItemCleanupPair<typeof item>>();
    const error = new Error('rejected');
    const [transposed, cleanup] = transpose(wrapPromise(pair.promise));

    expect(() => cleanup()).not.toThrow();
    pair.reject(error);
    await expect(transposed.promise).rejects.toBe(error);
    expect(getPromiseState(transposed)).toEqual({ kind: 'Err', error });
  });

  test('a second cleanup before the resolve throws', async () => {
    const pair = deferred<ItemCleanupPair<typeof item>>();
    const innerCleanup = vi.fn();
    const [transposed, cleanup] = transpose(wrapPromise(pair.promise));

    cleanup();
    expect(() => cleanup()).toThrow(message);

    pair.resolve([item, innerCleanup]);
    await transposed.promise;
    expect(innerCleanup).toHaveBeenCalledTimes(1);
  });

  test('a second cleanup after the resolve throws', async () => {
    const pair = deferred<ItemCleanupPair<typeof item>>();
    const innerCleanup = vi.fn();
    const [transposed, cleanup] = transpose(wrapPromise(pair.promise));

    pair.resolve([item, innerCleanup]);
    await transposed.promise;
    cleanup();
    expect(() => cleanup()).toThrow(message);
    expect(innerCleanup).toHaveBeenCalledTimes(1);
  });

  test('an already-resolved wrapper transposes synchronously', async () => {
    const innerCleanup = vi.fn();
    const [transposed, cleanup] = transpose(
      wrapResolvedValue<ItemCleanupPair<typeof item>>([item, innerCleanup]),
    );
    expect(transposed.result).toEqual({ kind: 'Ok', value: item });

    cleanup();
    expect(innerCleanup).toHaveBeenCalledTimes(1);

    await transposed.promise;
    expect(innerCleanup).toHaveBeenCalledTimes(1);
  });
});
