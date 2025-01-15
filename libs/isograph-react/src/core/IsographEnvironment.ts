import { ParentCache } from '@isograph/react-disposable-state';
import { RetainedQuery } from './garbageCollection';
import { WithEncounteredRecords } from './read';
import { FragmentReference, Variables } from './FragmentReference';
import { PromiseWrapper, wrapPromise } from './PromiseWrapper';
import { IsographEntrypoint } from './entrypoint';
import type { ReaderAst } from './reader';
import { LogFunction, WrappedLogFunction } from './logging';

export type ComponentOrFieldName = string;
export type StringifiedArgs = string;
type ComponentCache = {
  [key: DataId]: {
    [key: ComponentOrFieldName]: { [key: StringifiedArgs]: React.FC<any> };
  };
};

export type FragmentSubscription<
  TReadFromStore extends { parameters: object; data: object },
> = {
  readonly kind: 'FragmentSubscription';
  readonly callback: (
    newEncounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  ) => void;
  /** The value read out from the previous call to readButDoNotEvaluate */
  readonly encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>;
  readonly fragmentReference: FragmentReference<TReadFromStore, any>;
  readonly readerAst: ReaderAst<TReadFromStore>;
};

type AnyChangesToRecordSubscription = {
  readonly kind: 'AnyChangesToRecord';
  readonly callback: () => void;
  readonly recordLink: Link;
};

type AnyRecordSubscription = {
  readonly kind: 'AnyRecords';
  readonly callback: () => void;
};

type Subscription =
  | FragmentSubscription<{ parameters: object; data: object }>
  | AnyChangesToRecordSubscription
  | AnyRecordSubscription;
type Subscriptions = Set<Subscription>;
// Should this be a map?
type CacheMap<T> = { [index: string]: ParentCache<T> };

export type IsographEnvironment = {
  readonly store: IsographStore;
  readonly networkFunction: IsographNetworkFunction;
  readonly missingFieldHandler: MissingFieldHandler | null;
  readonly componentCache: ComponentCache;
  readonly subscriptions: Subscriptions;
  // N.B. this must be <any, any>, but all *usages* of this should go through
  // a function that adds type parameters.
  readonly fragmentCache: CacheMap<FragmentReference<any, any>>;
  // TODO make this a CacheMap and add GC
  readonly entrypointArtifactCache: Map<
    string,
    PromiseWrapper<IsographEntrypoint<any, any>>
  >;
  readonly retainedQueries: Set<RetainedQuery>;
  readonly gcBuffer: Array<RetainedQuery>;
  readonly gcBufferSize: number;
  readonly loggers: Set<WrappedLogFunction>;
};

export type MissingFieldHandler = (
  storeRecord: StoreRecord,
  root: Link,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: Variables | null,
) => Link | undefined;

export type IsographNetworkFunction = (
  queryText: string,
  variables: Variables,
) => Promise<any>;

export type Link = {
  readonly __link: DataId;
  readonly __typename: TypeName;
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
  readonly id?: DataId;
};

export type TypeName = string;
export type DataId = string;

export const ROOT_ID: DataId & '__ROOT' = '__ROOT';

export type IsographStore = {
  [index: TypeName]: {
    [index: DataId]: StoreRecord | null;
  } | null;
  readonly Query: {
    readonly __ROOT: StoreRecord;
  };
};

const DEFAULT_GC_BUFFER_SIZE = 10;
export function createIsographEnvironment(
  store: IsographStore,
  networkFunction: IsographNetworkFunction,
  missingFieldHandler?: MissingFieldHandler | null,
  logFunction?: LogFunction | null,
): IsographEnvironment {
  return {
    store,
    networkFunction,
    missingFieldHandler: missingFieldHandler ?? null,
    componentCache: {},
    subscriptions: new Set(),
    fragmentCache: {},
    entrypointArtifactCache: new Map(),
    retainedQueries: new Set(),
    gcBuffer: [],
    gcBufferSize: DEFAULT_GC_BUFFER_SIZE,
    loggers: logFunction != null ? new Set([{ log: logFunction }]) : new Set(),
  };
}

export function createIsographStore(): IsographStore {
  return {
    Query: {
      [ROOT_ID]: {},
    },
  };
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
    '__link' in maybeLink &&
    maybeLink.__link != null &&
    '__typename' in maybeLink &&
    maybeLink.__typename != null
  ) {
    return maybeLink;
  }
  return null;
}

export function getOrLoadIsographArtifact(
  environment: IsographEnvironment,
  key: string,
  loader: () => Promise<IsographEntrypoint<any, any>>,
): PromiseWrapper<IsographEntrypoint<any, any>> {
  const value = environment.entrypointArtifactCache.get(key);
  if (value != null) {
    return value;
  }
  const wrapped = wrapPromise(loader());
  environment.entrypointArtifactCache.set(key, wrapped);
  return wrapped;
}
