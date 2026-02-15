import { describe, test, vi, expect } from 'vitest';
import {
  createIsographStore,
  createIsographEnvironment,
  callSubscriptions,
} from '..';

describe('subscribe', () => {
  describe('callSubscriptions', () => {
    describe('AnyChangesToRecord', () => {
      test('should call the callback if the record has changed', () => {
        const networkFunction = vi
          .fn()
          .mockRejectedValue(new Error('Fetch failed'));
        const environment = createIsographEnvironment(
          createIsographStore(),
          networkFunction,
        );
        const callback = vi.fn();
        environment.subscriptions.add({
          kind: 'AnyChangesToRecord',
          callback,
          recordLink: {
            __link: '1',
            __typename: 'User',
          },
        });

        callSubscriptions(environment, new Map([['User', new Set(['1'])]]));

        expect(callback).toHaveBeenCalled();
      });

      test('should not call the callback if the record has not changed', () => {
        const networkFunction = vi
          .fn()
          .mockRejectedValue(new Error('Fetch failed'));
        const environment = createIsographEnvironment(
          createIsographStore(),
          networkFunction,
        );
        const callback = vi.fn();
        environment.subscriptions.add({
          kind: 'AnyChangesToRecord',
          callback,
          recordLink: {
            __link: '1',
            __typename: 'User',
          },
        });

        callSubscriptions(environment, new Map([['User', new Set(['2'])]]));

        expect(callback).not.toHaveBeenCalled();
      });
    });
  });
});
