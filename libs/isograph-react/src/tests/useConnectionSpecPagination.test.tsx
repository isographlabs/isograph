import { act, render } from '@testing-library/react';
import React, { StrictMode } from 'react';
import { describe, expect, test } from 'vitest';
import { createIsographStore } from '../core/IsographEnvironment';
import { callSubscriptions } from '../core/subscribe';
import { createIsographEnvironment } from '../react/createIsographEnvironment';
import { IsographEnvironmentProvider } from '../react/IsographEnvironmentProvider';
import {
  anyRecordsCount,
  createConnectionLoadable,
  firstRequest,
  type Item,
} from './paginationHarness';
import {
  useConnectionSpecPagination,
  type UsePaginationReturnValue,
} from '../loadable-hooks/useConnectionSpecPagination';

function renderConnectionPagination() {
  const { loadable, loads, requests } = createConnectionLoadable();
  const environment = createIsographEnvironment(createIsographStore(), () =>
    Promise.reject(new Error('unused network')),
  );
  const latest: {
    current: UsePaginationReturnValue<
      { data: object; parameters: object },
      Item
    >;
  } = {
    current: {
      kind: 'NoMoreRecords',
      results: [],
    },
  };

  function Host() {
    const pagination = useConnectionSpecPagination(loadable);
    latest.current = pagination;
    return <div>{pagination.kind}</div>;
  }

  const view = render(
    <StrictMode>
      <IsographEnvironmentProvider environment={environment}>
        <Host />
      </IsographEnvironmentProvider>
    </StrictMode>,
  );

  return { environment, latest, loads, requests, unmount: view.unmount };
}

describe('useConnectionSpecPagination', () => {
  // Fails on main: both fetchMore calls read the same render's list, so each
  // calls the loadable field with after: null and starts a request for the same
  // page. Passes once the hook is rebuilt on useOneAtATimeRequests.
  test.fails(
    'a second fetchMore before a re-render does not start a second request',
    () => {
      const { latest, loads, unmount } = renderConnectionPagination();
      act(() => {
        const pagination = latest.current;
        if (pagination.kind !== 'HasMoreRecords') {
          throw new Error(`expected HasMoreRecords, got ${pagination.kind}`);
        }
        pagination.fetchMore(2);
        pagination.fetchMore(2);
      });
      expect(loads).toEqual([{ after: null, first: 2 }]);
      unmount();
    },
  );

  // Fails on main: calling a fetchMore again after its first call's list
  // committed clones page pointers that the commit disposed, and throws "This
  // reference counted pointer has already been disposed". Passes once the hook
  // is rebuilt on useOneAtATimeRequests.
  test.fails(
    'a stale fetchMore after the list was replaced is a no-op',
    async () => {
      const { environment, latest, requests, unmount } =
        renderConnectionPagination();
      act(() => {
        const pagination = latest.current;
        if (pagination.kind !== 'HasMoreRecords') {
          throw new Error(`expected HasMoreRecords, got ${pagination.kind}`);
        }
        pagination.fetchMore(2);
      });
      await act(async () => {
        firstRequest(requests).resolve();
        callSubscriptions(environment, new Map());
      });
      let fetchMoreFromFirstPage: ((count: number) => void) | null = null;
      act(() => {
        const pagination = latest.current;
        if (pagination.kind !== 'HasMoreRecords') {
          throw new Error(`expected HasMoreRecords, got ${pagination.kind}`);
        }
        fetchMoreFromFirstPage = pagination.fetchMore;
        pagination.fetchMore(2);
      });
      expect(() => {
        if (fetchMoreFromFirstPage == null) {
          throw new Error('missing fetchMore');
        }
        fetchMoreFromFirstPage(2);
      }).not.toThrow();
      unmount();
    },
  );

  // Fails on main: the rejected request writes nothing to the store, so the
  // subscribeToAnyChange callback never re-renders the hook and the hook stays
  // Pending. Passes once the hook is rebuilt on useOneAtATimeRequests.
  test.fails('a failed page does not stay Pending', async () => {
    const { latest, requests, unmount } = renderConnectionPagination();
    act(() => {
      const pagination = latest.current;
      if (pagination.kind !== 'HasMoreRecords') {
        throw new Error(`expected HasMoreRecords, got ${pagination.kind}`);
      }
      pagination.fetchMore(2);
    });
    expect(latest.current.kind).toBe('Pending');
    await act(async () => {
      firstRequest(requests).reject(new Error('network'));
    });
    expect(latest.current.kind).not.toBe('Pending');
    unmount();
  });

  // Fails on main: each Pending render's subscribeToAnyChange subscription is
  // removed only by its own callback, which runs only on a store write, so
  // after the rejection the subscriptions stay in environment.subscriptions.
  // Passes once the hook is rebuilt on useOneAtATimeRequests.
  test.fails(
    'a failed request without a store write does not leave an AnyRecords subscription',
    async () => {
      const { environment, latest, requests, unmount } =
        renderConnectionPagination();
      act(() => {
        const pagination = latest.current;
        if (pagination.kind !== 'HasMoreRecords') {
          throw new Error(`expected HasMoreRecords, got ${pagination.kind}`);
        }
        pagination.fetchMore(2);
      });
      expect(anyRecordsCount(environment)).toBeGreaterThan(0);
      await act(async () => {
        firstRequest(requests).reject(new Error('network'));
      });
      expect(anyRecordsCount(environment)).toBe(0);
      unmount();
    },
  );

  // Fails on main: when a store change re-renders the hook after the rejection,
  // the hook throws the network error from render, with no error result and no
  // retry. Passes once the hook is rebuilt on useOneAtATimeRequests.
  test.fails('a failed page is a returned error, not a throw', async () => {
    const { environment, latest, requests, unmount } =
      renderConnectionPagination();
    act(() => {
      const pagination = latest.current;
      if (pagination.kind !== 'HasMoreRecords') {
        throw new Error(`expected HasMoreRecords, got ${pagination.kind}`);
      }
      pagination.fetchMore(2);
    });
    let thrown: unknown;
    try {
      await act(async () => {
        firstRequest(requests).reject(new Error('network'));
        callSubscriptions(environment, new Map());
      });
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeUndefined();
    expect(latest.current.kind).not.toBe('Pending');
    unmount();
  });
});
