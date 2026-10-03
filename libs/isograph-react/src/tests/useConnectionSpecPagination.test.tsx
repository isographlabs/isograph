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
  // doubled fetchMore starts two loadableField requests for the same cursor
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

  // stale fetchMore clones pages the primitive has already disposed
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

  // failed request writes nothing, so subscribeToAnyChange never rerenders
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

  // subscribeToAnyChange during render is only removed in its callback
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

  // Err throws; there is no returned error status and no retry
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
