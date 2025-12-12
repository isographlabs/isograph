import { describe, expect, test, vitest } from 'vitest';
import {
  getPromiseState,
  PromiseWrapperAll,
  PromiseWrapperThen,
  wrapPromise,
  wrapRejectedValue,
  wrapResolvedValue,
  type PromiseWrapper,
  type PromiseWrapperErr,
  type PromiseWrapperOk,
} from '../core/PromiseWrapper';

async function expectPromiseWrapperToEqual<T, E extends string>(
  received: PromiseWrapper<T, unknown>,
  expected: PromiseWrapperOk<T, E> | PromiseWrapperErr<T, E>,
) {
  const state = getPromiseState(expected);
  switch (state.kind) {
    case 'Ok': {
      await expect(received.promise).resolves.toEqual(state.value);
      break;
    }
    case 'Err': {
      await expect(received.promise).rejects.toThrow(state.error);
      break;
    }
  }
  expect(received.result).toEqual(expected.result);
}

describe('PromiseWrapper', () => {
  describe('PromiseWrapperThen', () => {
    describe('sync', () => {
      test('returns Pending for Pending', () => {
        expect(
          PromiseWrapperThen(
            wrapPromise(new Promise(() => {})),
            vitest.fn().mockRejectedValue('Unreachable'),
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
        ).toEqual(wrapPromise(new Promise(() => {})));
      });

      test('maps Ok to Ok', () => {
        expect(
          PromiseWrapperThen(
            wrapResolvedValue('foo'),
            (value) => value.toUpperCase(),
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
        ).toEqual(wrapResolvedValue('FOO'));
      });

      test('maps Err to Ok', () => {
        expect(
          PromiseWrapperThen(
            wrapRejectedValue('foo'),
            vitest.fn().mockRejectedValue('Unreachable'),
            (value) => value.toUpperCase(),
          ),
        ).toEqual(wrapResolvedValue('FOO'));
      });
    });

    describe('Pending', () => {
      test('maps Ok to Ok', () => {
        expectPromiseWrapperToEqual(
          PromiseWrapperThen(
            wrapPromise(Promise.resolve('foo')),
            (value) => value.toUpperCase(),
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
          wrapResolvedValue('FOO'),
        );
      });

      test('maps Err to Ok', () => {
        expectPromiseWrapperToEqual(
          PromiseWrapperThen(
            wrapPromise(Promise.reject('foo')) as PromiseWrapper<never, 'foo'>,
            vitest.fn().mockRejectedValue('Unreachable'),
            (value) => value.toUpperCase(),
          ),
          wrapResolvedValue('FOO'),
        );
      });
    });
  });
  describe('PromiseWrapperAll', () => {
    describe('sync', () => {
      test('returns Ok for empty', () => {
        expect(PromiseWrapperAll([])).toEqual(wrapResolvedValue([]));
      });

      test('returns Ok for all Ok', () => {
        expect(
          PromiseWrapperAll([
            wrapResolvedValue('foo'),
            wrapResolvedValue('bar'),
          ]),
        ).toEqual(wrapResolvedValue(['foo', 'bar']));
      });

      test('raw values pass through', () => {
        expect(
          PromiseWrapperAll([
            wrapResolvedValue('foo'),
            null,
            wrapResolvedValue('bar'),
          ]),
        ).toEqual(wrapResolvedValue(['foo', null, 'bar']));
      });

      test('returns Err for any Err', () => {
        expect(
          PromiseWrapperAll([
            wrapResolvedValue('foo'),
            wrapRejectedValue('bar'),
          ]),
        ).toEqual(wrapRejectedValue('bar'));
      });

      test('returns Pending if any Pending', () => {
        expect(
          PromiseWrapperAll([
            wrapPromise(new Promise(() => {})),
            wrapResolvedValue('bar'),
          ]),
        ).toEqual(wrapPromise(new Promise(() => {})));
      });
    });

    describe('Pending', () => {
      test('resolves to Ok', async () => {
        expect.assertions(2);

        const wrapper = PromiseWrapperAll([
          wrapPromise(Promise.resolve('foo')),
          wrapResolvedValue('bar'),
        ]);

        await expectPromiseWrapperToEqual(
          wrapper,
          wrapResolvedValue(['foo', 'bar']),
        );
      });

      test('resolves to Err', async () => {
        expect.assertions(2);

        const wrapper = PromiseWrapperAll([
          wrapPromise(Promise.reject('foo')),
          wrapResolvedValue('bar'),
        ]);

        await expectPromiseWrapperToEqual(wrapper, wrapRejectedValue('foo'));
      });
    });
  });
});
