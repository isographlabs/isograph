import { ParentCache } from '@isograph/disposable-types';
import { IsographEntrypoint } from './entrypoint';
import {
  ExtractStartUpdate,
  FragmentReference,
  Variables,
  type StableIdForFragmentReference,
  type UnknownTReadFromStore,
} from './FragmentReference';
import { RetainedQuery } from './garbageCollection';
import { LogFunction, WrappedLogFunction } from './logging';
import { PromiseWrapper, wrapPromise } from './PromiseWrapper';
import { NetworkRequestReaderOptions, WithEncounteredRecords } from './read';
import type { ReaderAst, StartUpdate } from './reader';

export type ComponentOrFieldName = string;
export type StringifiedArgs = string;

export type FieldCache<T> = {
  [key: StableIdForFragmentReference]: T;
};

export type FragmentSubscription<TReadFromStore extends UnknownTReadFromStore> =
  {
    readonly kind: 'FragmentSubscription';
    readonly callback: (
      newEncounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
    ) => void;
    /** The value read out from the previous call to readButDoNotEvaluate */
    readonly encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>;
    readonly fragmentReference: FragmentReference<TReadFromStore, any>;
    readonly readerAst: ReaderAst<TReadFromStore>;
  };

export type AnyChangesToRecordSubscription = {
  readonly kind: 'AnyChangesToRecord';
  readonly callback: () => void;
  readonly recordLink: Link;
};

export type AnyRecordSubscription = {
  readonly kind: 'AnyRecords';
  readonly callback: () => void;
};

export type Subscription =
  | FragmentSubscription<any>
  | AnyChangesToRecordSubscription
  | AnyRecordSubscription;
export type Subscriptions = Set<Subscription>;
// Should this be a map?
export type CacheMap<T> = { [index: string]: ParentCache<T> };

export type IsographEnvironment<TComponent = unknown> = {
  readonly store: IsographStore;
  readonly networkFunction: IsographNetworkFunction;
  readonly componentFunction: IsographComponentFunction<TComponent>;
  readonly missingFieldHandler: MissingFieldHandler | null;
  readonly componentCache: FieldCache<TComponent>;
  readonly eagerReaderCache: FieldCache<StartUpdate<any> | undefined>;
  readonly subscriptions: Subscriptions;
  // N.B. this must be <any, any>, but all *usages* of this should go through
  // a function that adds type parameters.
  readonly fragmentCache: CacheMap<FragmentReference<any, any>>;
  // TODO make this a CacheMap and add GC
  readonly entrypointArtifactCache: Map<
    string,
    PromiseWrapper<IsographEntrypoint<any, any, any>>
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

export type IsographComponentFunction<
  TComponent,
  TReadFromStore extends UnknownTReadFromStore = any,
> = (
  environment: IsographEnvironment,
  componentName: string,
  fragmentReference: FragmentReference<TReadFromStore, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  startUpdate: ExtractStartUpdate<TReadFromStore>,
) => TComponent;

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
export function createIsographEnvironment<TComponent>(
  store: IsographStore,
  networkFunction: IsographNetworkFunction,
  componentFunction: IsographComponentFunction<TComponent>,
  missingFieldHandler?: MissingFieldHandler | null,
  logFunction?: LogFunction | null,
): IsographEnvironment<TComponent> {
  logFunction?.({
    kind: 'EnvironmentCreated',
  });
  return {
    store,
    networkFunction,
    componentFunction,
    missingFieldHandler: missingFieldHandler ?? null,
    componentCache: {},
    eagerReaderCache: {},
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
  loader: () => Promise<IsographEntrypoint<any, any, any>>,
): PromiseWrapper<IsographEntrypoint<any, any, any>> {
  const value = environment.entrypointArtifactCache.get(key);
  if (value != null) {
    return value;
  }
  const wrapped = wrapPromise(loader());
  environment.entrypointArtifactCache.set(key, wrapped);
  return wrapped;
}
