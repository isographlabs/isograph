import { ReactNode, createContext, useContext } from 'react';
import * as React from 'react';
import { ParentCache } from '@isograph/isograph-react-disposable-state';
import { NormalizationAst } from './index';
import { getParentRecordKey } from './cache';

export const IsographEnvironmentContext =
  createContext<IsographEnvironment | null>(null);

type ComponentName = string;
type StringifiedArgs = string;
type ComponentCache = {
  [key: DataId]: {
    [key: ComponentName]: { [key: StringifiedArgs]: React.FC<any> };
  };
};

export type Subscriptions = Set<() => void>;
type SuspenseCache = { [index: string]: ParentCache<any> };

export type RetainedQuery = {
  normalizationAst: NormalizationAst;
  variables: {};
};

export type IsographEnvironment = {
  store: IsographStore;
  networkFunction: IsographNetworkFunction;
  missingFieldHandler: MissingFieldHandler | null;
  componentCache: ComponentCache;
  subscriptions: Subscriptions;
  suspenseCache: SuspenseCache;
  retainedQueries: Set<RetainedQuery>;
  gcBuffer: Array<RetainedQuery>;
  gcBufferSize: number;
};

export type MissingFieldHandler = (
  storeRecord: StoreRecord,
  root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null,
) => Link | undefined;

export type IsographNetworkFunction = (
  queryText: string,
  variables: object,
) => Promise<any>;

export type Link = {
  __link: DataId;
};
export type DataTypeValue =
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the store.
  | undefined
  // Singular scalar fields:
  | number
  | boolean
  | string
  | null
  // Singular linked fields:
  | Link
  // Plural scalar and linked fields:
  | DataTypeValue[];

export type StoreRecord = {
  [index: DataId | string]: DataTypeValue;
  // TODO __typename?: T, which is restricted to being a concrete string
  // TODO this shouldn't always be named id
  id?: DataId;
};

export type DataId = string;

export const ROOT_ID: DataId & '__ROOT' = '__ROOT';

export type IsographStore = {
  [index: DataId]: StoreRecord | null;
  __ROOT: StoreRecord;
};

export type IsographEnvironmentProviderProps = {
  environment: IsographEnvironment;
  children: ReactNode;
};

export function IsographEnvironmentProvider({
  environment,
  children,
}: IsographEnvironmentProviderProps) {
  return (
    <IsographEnvironmentContext.Provider value={environment}>
      {children}
    </IsographEnvironmentContext.Provider>
  );
}

export function useIsographEnvironment(): IsographEnvironment {
  const context = useContext(IsographEnvironmentContext);
  if (context == null) {
    throw new Error(
      'Unexpected null environment context. Make sure to render ' +
        'this component within an IsographEnvironmentProvider component',
    );
  }
  return context;
}

const DEFAULT_GC_BUFFER_SIZE = 10;
export function createIsographEnvironment(
  store: IsographStore,
  networkFunction: IsographNetworkFunction,
  missingFieldHandler?: MissingFieldHandler,
): IsographEnvironment {
  return {
    store,
    networkFunction,
    missingFieldHandler: missingFieldHandler ?? null,
    componentCache: {},
    subscriptions: new Set(),
    suspenseCache: {},
    retainedQueries: new Set(),
    gcBuffer: [],
    gcBufferSize: DEFAULT_GC_BUFFER_SIZE,
  };
}

export function createIsographStore() {
  return {
    [ROOT_ID]: {},
  };
}

export function garbageCollectEnvironment(environment: IsographEnvironment) {
  const retainedIds = new Set<DataId>([ROOT_ID]);

  for (const query of environment.retainedQueries) {
    recordReachableIds(environment.store, query, retainedIds);
  }
  for (const query of environment.gcBuffer) {
    recordReachableIds(environment.store, query, retainedIds);
  }

  for (const dataId in environment.store) {
    if (!retainedIds.has(dataId)) {
      delete environment.store[dataId];
    }
  }
}

function recordReachableIds(
  store: IsographStore,
  retainedQuery: RetainedQuery,
  mutableRetainedIds: Set<DataId>,
) {
  recordReachableIdsFromRecord(
    store,
    store[ROOT_ID],
    mutableRetainedIds,
    retainedQuery.normalizationAst,
    retainedQuery.variables,
  );
}

function getLinkedId(data: Exclude<DataTypeValue, null | void>): string {
  // @ts-expect-error
  if (data.__link != null) {
    // @ts-expect-error
    return data.__link;
  } else {
    throw new Error('Record in an invalid state');
  }
}

function recordReachableIdsFromRecord(
  store: IsographStore,
  currentRecord: StoreRecord,
  mutableRetainedIds: Set<DataId>,
  selections: NormalizationAst,
  variables: { [index: string]: string } | null,
) {
  for (const selection of selections) {
    switch (selection.kind) {
      case 'Linked':
        const linkKey = getParentRecordKey(selection, variables ?? {});
        const linkedFieldOrFields = currentRecord[linkKey];

        const ids = [];
        if (Array.isArray(linkedFieldOrFields)) {
          for (const link of linkedFieldOrFields) {
            if (link != null) {
              const id = getLinkedId(link);
              ids.push(id);
            }
          }
        } else {
          if (linkedFieldOrFields != null) {
            const id = getLinkedId(linkedFieldOrFields);
            ids.push(id);
          }
        }

        for (const nextRecordId of ids) {
          const nextRecord = store[nextRecordId];
          if (nextRecord != null) {
            mutableRetainedIds.add(nextRecordId);
            recordReachableIdsFromRecord(
              store,
              nextRecord,
              mutableRetainedIds,
              selection.selections,
              variables,
            );
          }
        }

        continue;
      case 'Scalar':
        continue;
    }
  }
}

type DidUnretainSomeQuery = boolean;
export function unretainQuery(
  environment: IsographEnvironment,
  retainedQuery: RetainedQuery,
): DidUnretainSomeQuery {
  environment.retainedQueries.delete(retainedQuery);
  environment.gcBuffer.push(retainedQuery);

  if (environment.gcBuffer.length > environment.gcBufferSize) {
    environment.gcBuffer.shift();
    return true;
  }

  return false;
}

export function retainQuery(
  environment: IsographEnvironment,
  queryToRetain: RetainedQuery,
) {
  environment.retainedQueries.add(queryToRetain);
  // TODO can we remove this query from the buffer somehow?
  // We are relying on === equality, but we really should be comparing
  // id + variables
}
