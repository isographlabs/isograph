import { act, render } from '@testing-library/react';
import React, { StrictMode } from 'react';
import { describe, expect, test } from 'vitest';
import { createIsographStore } from '../core/IsographEnvironment';
import { callSubscriptions } from '../core/subscribe';
import { createIsographEnvironment } from '../react/createIsographEnvironment';
import { IsographEnvironmentProvider } from '../react/IsographEnvironmentProvider';
import {
  createSkipLimitLoadable,
  firstRequest,
  type Item,
} from './paginationHarness';
import {
  useSkipLimitPagination,
  type UseSkipLimitReturnValue,
} from '../loadable-hooks/useSkipLimitPagination';

function renderSkipLimitPagination() {
  const { loadable, loads, requests } = createSkipLimitLoadable();
  const environment = createIsographEnvironment(createIsographStore(), () =>
    Promise.reject(new Error('unused network')),
  );
  const latest: {
    current: UseSkipLimitReturnValue<
      { data: object; parameters: object },
      Item
    >;
  } = {
    current: {
      kind: 'Complete',
      results: [],
      fetchMore: () => {},
    },
  };

  function Host() {
    const pagination = useSkipLimitPagination(loadable);
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

describe('useSkipLimitPagination', () => {
  // Calling a fetchMore again after its first call's list committed reads page
  // pointers that the commit released. The call does nothing: it does not throw
  // and starts no request.
  test('a stale fetchMore after the list was replaced is a no-op', async () => {
    const { environment, latest, loads, requests, unmount } =
      renderSkipLimitPagination();
    act(() => {
      const pagination = latest.current;
      if (pagination.kind !== 'Complete') {
        throw new Error(`expected Complete, got ${pagination.kind}`);
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
      if (pagination.kind !== 'Complete') {
        throw new Error(`expected Complete, got ${pagination.kind}`);
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
      { skip: 0, limit: 2 },
      { skip: 1, limit: 2 },
    ]);
    unmount();
  });
});
