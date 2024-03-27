import { ParentCache } from '@isograph/isograph-react-disposable-state';
import { RetainedQuery } from './garbageCollection';

type ComponentName = string;
type StringifiedArgs = string;
type ComponentCache = {
  [key: DataId]: {
    [key: ComponentName]: { [key: StringifiedArgs]: React.FC<any> };
  };
};

export type Subscriptions = Set<() => void>;
type SuspenseCache = { [index: string]: ParentCache<any> };

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

export function defaultMissingFieldHandler(
  _storeRecord: StoreRecord,
  _root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null,
): Link | undefined {
  if (fieldName === 'node' || fieldName === 'user') {
    const variable = arguments_?.['id'];
    const value = variables?.[variable];

    // TODO can we handle explicit nulls here too? Probably, after wrapping in objects
    if (value != null) {
      return { __link: value };
    }
  }
}

export function assertLink(link: DataTypeValue): Link | null {
  if (Array.isArray(link)) {
    throw new Error('Unexpected array');
  }
  if (link == null) {
    return null;
  }
  if (typeof link === 'object') {
    return link;
  }
  throw new Error('Invalid link');
}
