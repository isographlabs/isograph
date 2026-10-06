import type { FragmentReference } from '../core/FragmentReference';
import { ROOT_ID } from '../core/IsographEnvironment';
import {
  type PromiseWrapper,
  wrapPromise,
  wrapResolvedValue,
} from '../core/PromiseWrapper';
import type { LoadableField } from '../core/reader';
import type { Connection } from '../loadable-hooks/useConnectionSpecPagination';
import type { UseConnectionSpecPaginationArgs } from '../loadable-hooks/useConnectionSpecPagination';
import type { UseSkipLimitPaginationArgs } from '../loadable-hooks/useSkipLimitPagination';

export type Item = { readonly id: string };

type PageStore = {
  parameters: object;
  data: object;
};

export type PendingRequest = {
  resolve: () => void;
  reject: (error: Error) => void;
};

function fragmentReference<TResult>(
  networkRequest: FragmentReference<PageStore, TResult>['networkRequest'],
  resolver: () => TResult,
): FragmentReference<PageStore, TResult> {
  return {
    kind: 'FragmentReference',
    readerWithRefetchQueries: wrapResolvedValue({
      kind: 'ReaderWithRefetchQueries',
      readerArtifact: {
        kind: 'EagerReaderArtifact',
        fieldName: 'page',
        readerAst: [],
        resolver: () => resolver(),
        hasUpdatable: false,
      },
      nestedRefetchQueries: [],
    }),
    root: { __link: ROOT_ID, __typename: 'Query' },
    fieldName: 'page',
    readerArtifactKind: 'EagerReaderArtifact',
    variables: {},
    networkRequest,
  };
}

function createPendingNetwork(): {
  wrapper: PromiseWrapper<void>;
  request: PendingRequest;
} {
  let resolve!: () => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = () => {
      res();
    };
    reject = (error) => {
      rej(error);
    };
  });
  promise.catch(() => {});
  return {
    wrapper: wrapPromise<void>(promise),
    request: { resolve, reject },
  };
}

export function firstRequest(requests: PendingRequest[]): PendingRequest {
  const request = requests[0];
  if (request == null) {
    throw new Error('expected a started request');
  }
  return request;
}

export function createConnectionLoadable(): {
  loadable: LoadableField<
    PageStore,
    Connection<Item>,
    UseConnectionSpecPaginationArgs
  >;
  loads: UseConnectionSpecPaginationArgs[];
  requests: PendingRequest[];
} {
  const loads: UseConnectionSpecPaginationArgs[] = [];
  const requests: PendingRequest[] = [];
  const loadable: LoadableField<
    PageStore,
    Connection<Item>,
    UseConnectionSpecPaginationArgs
  > = (args) => {
    const typedArgs = args as UseConnectionSpecPaginationArgs;
    loads.push(typedArgs);
    return [
      `connection-${loads.length}`,
      () => {
        const { wrapper, request } = createPendingNetwork();
        requests.push(request);
        return [
          fragmentReference(wrapper, () => ({
            edges: [{ id: `item-${loads.length}` }],
            pageInfo: {
              hasNextPage: true,
              endCursor: `cursor-${loads.length}`,
            },
          })),
          () => {},
        ];
      },
    ];
  };
  return { loadable, loads, requests };
}

export function createSkipLimitLoadable(): {
  loadable: LoadableField<
    PageStore,
    ReadonlyArray<Item>,
    UseSkipLimitPaginationArgs
  >;
  loads: UseSkipLimitPaginationArgs[];
  requests: PendingRequest[];
} {
  const loads: UseSkipLimitPaginationArgs[] = [];
  const requests: PendingRequest[] = [];
  const loadable: LoadableField<
    PageStore,
    ReadonlyArray<Item>,
    UseSkipLimitPaginationArgs
  > = (args) => {
    const typedArgs = args as UseSkipLimitPaginationArgs;
    loads.push(typedArgs);
    return [
      `skip-limit-${loads.length}`,
      () => {
        const { wrapper, request } = createPendingNetwork();
        requests.push(request);
        return [
          fragmentReference(wrapper, () => [{ id: `item-${loads.length}` }]),
          () => {},
        ];
      },
    ];
  };
  return { loadable, loads, requests };
}
