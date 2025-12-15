import { describe, expect, test, vitest } from 'vitest';
import {
  getPromiseState,
  wrapPromise,
  wrapRejectedValue,
  wrapResolvedValue,
  type PromiseWrapper,
  type PromiseWrapperErr,
  type PromiseWrapperOk,
} from '../core/PromiseWrapper';

import * as PromiseWrapperUtils from '../core/PromiseWrapper';

async function expectPromiseWrapperToEqual<T, E extends string>(
  received: PromiseWrapper<T, unknown>,
  expected: PromiseWrapperOk<T> | PromiseWrapperErr<E>,
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
    case 'Pending': {
    }
  }
  expect(received.result).toEqual(expected.result);
}

describe('PromiseWrapper', () => {
  describe('mapOk', () => {
    describe('sync', () => {
      test('returns Pending for Pending', () => {
        expect(
          PromiseWrapperUtils.mapOk(
            wrapPromise(new Promise(() => {})),
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
        ).toEqual(wrapPromise(new Promise(() => {})));
      });

      test('maps Ok to Ok', () => {
        expect(
          PromiseWrapperUtils.mapOk(wrapResolvedValue('foo'), (value) =>
            value.toUpperCase(),
          ),
        ).toEqual(wrapResolvedValue('FOO'));
      });

      test("doesn't map Err", () => {
        expect(
          PromiseWrapperUtils.mapOk(
            wrapRejectedValue('foo'),
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
        ).toEqual(wrapRejectedValue('foo'));
      });
    });

    describe('Pending', () => {
      test('maps Ok to Ok', async () => {
        expect.assertions(2);
        await expectPromiseWrapperToEqual(
          PromiseWrapperUtils.mapOk(
            wrapPromise(Promise.resolve('foo')),
            (value) => value.toUpperCase(),
          ),
          wrapResolvedValue('FOO'),
        );
      });

      test("doesn't map Err", async () => {
        expect.assertions(2);
        await expectPromiseWrapperToEqual(
          PromiseWrapperUtils.mapOk(
            wrapPromise(Promise.reject('foo')) as PromiseWrapper<never, 'foo'>,
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
          wrapRejectedValue('foo'),
        );
      });
    });
  });

  describe('mapErr', () => {
    describe('sync', () => {
      test('returns Pending for Pending', () => {
        expect(
          PromiseWrapperUtils.mapErr(
            wrapPromise(new Promise(() => {})),

            vitest.fn().mockRejectedValue('Unreachable'),
          ),
        ).toEqual(wrapPromise(new Promise(() => {})));
      });

      test("doesn't map Ok", () => {
        expect(
          PromiseWrapperUtils.mapErr(
            wrapResolvedValue('foo'),
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
        ).toEqual(wrapResolvedValue('foo'));
      });

      test('maps Err to Err', () => {
        expect(
          PromiseWrapperUtils.mapErr(wrapRejectedValue('foo'), (value) =>
            value.toUpperCase(),
          ),
        ).toEqual(wrapRejectedValue('FOO'));
      });
    });

    describe('Pending', () => {
      test("doesn't map Ok", async () => {
        expect.assertions(2);
        await expectPromiseWrapperToEqual(
          PromiseWrapperUtils.mapErr(
            wrapPromise(Promise.resolve('foo')),
            vitest.fn().mockRejectedValue('Unreachable'),
          ),
          wrapResolvedValue('foo'),
        );
      });

      test('maps Err to Err', async () => {
        expect.assertions(2);
        await expectPromiseWrapperToEqual(
          PromiseWrapperUtils.mapErr(
            wrapPromise(Promise.reject('foo')) as PromiseWrapper<never, 'foo'>,
            (value) => value.toUpperCase(),
          ),
          wrapRejectedValue('FOO'),
        );
      });
    });
  });

  describe('all', () => {
    describe('sync', () => {
      test('returns Ok for empty', () => {
        expect(PromiseWrapperUtils.all([])).toEqual(wrapResolvedValue([]));
      });

      test('returns Ok for all Ok', () => {
        expect(
          PromiseWrapperUtils.all([
            wrapResolvedValue('foo'),
            wrapResolvedValue('bar'),
          ]),
        ).toEqual(wrapResolvedValue(['foo', 'bar']));
      });

      test('raw values pass through', () => {
        expect(
          PromiseWrapperUtils.all([
            wrapResolvedValue('foo'),
            null,
            wrapResolvedValue('bar'),
          ]),
        ).toEqual(wrapResolvedValue(['foo', null, 'bar']));
      });

      test('returns Err for any Err', () => {
        expect(
          PromiseWrapperUtils.all([
            wrapResolvedValue('foo'),
            wrapRejectedValue('bar'),
          ]),
        ).toEqual(wrapRejectedValue('bar'));
      });

      test('returns Pending if any Pending', () => {
        expect(
          PromiseWrapperUtils.all([
            wrapPromise(new Promise(() => {})),
            wrapResolvedValue('bar'),
          ]),
        ).toEqual(wrapPromise(new Promise(() => {})));
      });
    });

    describe('Pending', () => {
      test('resolves to Ok', async () => {
        expect.assertions(2);

        const wrapper = PromiseWrapperUtils.all([
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

        const wrapper = PromiseWrapperUtils.all([
          wrapPromise(Promise.reject('foo')),
          wrapResolvedValue('bar'),
        ]);

        await expectPromiseWrapperToEqual(wrapper, wrapRejectedValue('foo'));
      });
    });
  });
});
