import { ParentCache } from '@isograph/react-disposable-state';
import { RetainedQuery } from './garbageCollection';
import { WithEncounteredRecords } from './read';
import { FragmentReference } from './FragmentReference';

export type ComponentOrFieldName = string;
export type StringifiedArgs = string;
type ComponentCache = {
  [key: DataId]: {
    [key: ComponentOrFieldName]: { [key: StringifiedArgs]: React.FC<any> };
  };
};

type FragmentSubscription<TReadFromStore extends Object> = {
  readonly kind: 'FragmentSubscription';
  readonly callback: (
    newEncounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  ) => void;
  /** The value read out from the previous call to readButDoNotEvaluate */
  readonly encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>;
  readonly fragmentReference: FragmentReference<TReadFromStore, any>;
};
type AnyRecordSubscription = {
  readonly kind: 'AnyRecords';
  readonly callback: () => void;
};

type Subscription = FragmentSubscription<Object> | AnyRecordSubscription;
type Subscriptions = Set<Subscription>;
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

export function assertLink(link: DataTypeValue): Link | null | undefined {
  if (Array.isArray(link)) {
    throw new Error('Unexpected array');
  }
  if (link == null) {
    return link;
  }
  if (typeof link === 'object') {
    return link;
  }
  throw new Error('Invalid link');
}

export function getLink(maybeLink: DataTypeValue): Link | null {
  if (
    maybeLink != null &&
    typeof maybeLink === 'object' &&
    // @ts-expect-error this is safe
    maybeLink.__link != null
  ) {
    return maybeLink as any;
  }
  return null;
}
