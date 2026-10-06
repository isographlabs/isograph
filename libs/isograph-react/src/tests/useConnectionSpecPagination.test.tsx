import { act, render } from '@testing-library/react';
import React, { StrictMode } from 'react';
import { describe, expect, test } from 'vitest';
import { createIsographStore } from '../core/IsographEnvironment';
import { callSubscriptions } from '../core/subscribe';
import { createIsographEnvironment } from '../react/createIsographEnvironment';
import { IsographEnvironmentProvider } from '../react/IsographEnvironmentProvider';
import {
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
  // Calling a fetchMore again after its first call's list committed reads page
  // pointers that the commit released. The call does nothing: it does not throw
  // and starts no request.
  test('a stale fetchMore after the list was replaced is a no-op', async () => {
    const { environment, latest, loads, requests, unmount } =
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
    expect(loads).toEqual([
      { after: null, first: 2 },
      { after: 'cursor-1', first: 2 },
    ]);
    unmount();
  });
});
