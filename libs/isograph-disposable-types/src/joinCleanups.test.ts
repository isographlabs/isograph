import { assert, describe, expect, test } from 'vitest';
import { joinCleanups } from './index';

describe('joinCleanups', () => {
  test('Disposes in reverse argument order', () => {
    const order: string[] = [];
    const [items, dispose] = joinCleanups(
      [
        'a',
        () => {
          order.push('a');
        },
      ],
      [
        'b',
        () => {
          order.push('b');
        },
      ],
      [
        'c',
        () => {
          order.push('c');
        },
      ],
    );

    expect(items).toEqual(['a', 'b', 'c']);
    dispose();
    expect(order).toEqual(['c', 'b', 'a']);
  });

  test('If a cleanup throws, remaining cleanups still run and the first error is rethrown', () => {
    const order: string[] = [];
    const errorB = new Error('b');
    const [, dispose] = joinCleanups(
      [
        'a',
        () => {
          order.push('a');
        },
      ],
      [
        'b',
        () => {
          order.push('b');
          throw errorB;
        },
      ],
      [
        'c',
        () => {
          order.push('c');
        },
      ],
    );

    let thrown: unknown;
    try {
      dispose();
    } catch (e) {
      thrown = e;
    }

    assert(thrown === errorB);
    expect(order).toEqual(['c', 'b', 'a']);
  });

  test('If two cleanups throw, the first error in dispose order is rethrown', () => {
    const errorA = new Error('a');
    const errorC = new Error('c');
    const [, dispose] = joinCleanups(
      [
        'a',
        () => {
          throw errorA;
        },
      ],
      ['b', () => {}],
      [
        'c',
        () => {
          throw errorC;
        },
      ],
    );

    let thrown: unknown;
    try {
      dispose();
    } catch (e) {
      thrown = e;
    }

    assert(thrown === errorC);
  });
});
