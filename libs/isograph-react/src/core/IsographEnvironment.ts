import { ParentCache } from '@isograph/react-disposable-state';
import { RetainedQuery } from './garbageCollection';
import { WithEncounteredRecords } from './read';
import { FragmentReference, Variables } from './FragmentReference';
import { PromiseWrapper, wrapPromise } from './PromiseWrapper';
import { IsographEntrypoint } from './entrypoint';
import type { ReaderAst } from './reader';

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
type AnyRecordSubscription = {
  readonly kind: 'AnyRecords';
  readonly callback: () => void;
};

type Subscription =
  | FragmentSubscription<{ parameters: object; data: object }>
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
};

export type MissingFieldHandler = (
  storeRecord: StoreRecord,
  root: DataId,
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

export type DataId = string;

export const ROOT_ID: DataId & '__ROOT' = '__ROOT';

export type IsographStore = {
  [index: DataId]: StoreRecord | null;
  readonly __ROOT: StoreRecord;
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
    fragmentCache: {},
    entrypointArtifactCache: new Map(),
    retainedQueries: new Set(),
    gcBuffer: [],
    gcBufferSize: DEFAULT_GC_BUFFER_SIZE,
  };
}

export function createIsographStore(): IsographStore {
  return {
    [ROOT_ID]: {},
  };
}

export function defaultMissingFieldHandler(
  _storeRecord: StoreRecord,
  _root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: Variables | null,
): Link | undefined {
  if (fieldName === 'node' || fieldName === 'user') {
    const variable = arguments_?.['id'];
    const value = variables?.[variable];

    // TODO can we handle explicit nulls here too? Probably, after wrapping in objects
    if (value != null) {
      // @ts-expect-error
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
