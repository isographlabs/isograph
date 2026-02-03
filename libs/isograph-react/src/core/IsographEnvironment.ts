import type { ParentCache } from '@isograph/react-disposable-state';
import type { Brand } from './brand';
import type {
  LinkedParentRecordKey,
  ParentRecordKey,
  ScalarParentRecordKey,
} from './cache';
import type {
  IsographEntrypoint,
  IsographOperation,
  IsographPersistedOperation,
  RawReaderWithRefetchQueries,
  ReaderWithRefetchQueries,
  ReaderWithRefetchQueriesLoader,
} from './entrypoint';
import type {
  ExtractStartUpdate,
  FragmentReference,
  StableIdForFragmentReference,
  UnknownTReadFromStore,
  Variables,
} from './FragmentReference';
import type { RetainedQuery } from './garbageCollection';
import type { LogFunction, WrappedLogFunction } from './logging';
import { type StoreLayer } from './optimisticProxy';
import type { PromiseWrapper } from './PromiseWrapper';
import { wrapPromise, wrapResolvedValue } from './PromiseWrapper';
import type {
  NetworkRequestReaderOptions,
  WithEncounteredRecords,
} from './read';
import type { ReaderAst, StartUpdate } from './reader';
import { isArray } from './util';

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
    encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>;
    readonly fragmentReference: FragmentReference<TReadFromStore, any>;
    readonly readerAst: ReaderAst<TReadFromStore>;
  };

export type AnyChangesToRecordSubscription = {
  readonly kind: 'AnyChangesToRecord';
  readonly callback: () => void;
  readonly recordLink: StoreLink;
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

export type IsographEnvironment = {
  store: StoreLayer;
  readonly networkFunction: IsographNetworkFunction;
  readonly componentFunction: IsographComponentFunction;
  readonly missingFieldHandler: MissingFieldHandler | null;
  readonly componentCache: FieldCache<React.FC<any>>;
  readonly eagerReaderCache: FieldCache<StartUpdate<any> | undefined>;
  readonly subscriptions: Subscriptions;
  // N.B. this must be <any, any>, but all *usages* of this should go through
  // a function that adds type parameters.
  readonly fragmentCache: CacheMap<FragmentReference<any, any>>;
  // TODO make this a CacheMap and add GC
  readonly entrypointArtifactCache: Map<
    string,
    PromiseWrapper<IsographEntrypoint<any, any, any, any>>
  >;
  readonly retainedQueries: Set<RetainedQuery>;
  readonly gcBuffer: Array<RetainedQuery>;
  readonly gcBufferSize: number;
  readonly loggers: Set<WrappedLogFunction>;
};

export type MissingFieldHandler = (
  storeRecord: StoreRecord,
  root: StoreLink,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: Variables | null,
) => StoreLink | undefined;

export type IsographNetworkFunction = (
  operation: IsographOperation | IsographPersistedOperation,
  variables: Variables,
) => Promise<any>;

export type IsographComponentFunction = <
  TReadFromStore extends UnknownTReadFromStore = any,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  startUpdate: ExtractStartUpdate<TReadFromStore>,
) => React.FC<any>;

export interface Link<T extends TypeName> extends StoreLink {
  readonly __link: Brand<DataId, T>;
  readonly __typename: T;
}

export type StoreLink = {
  readonly __link: DataId;
  readonly __typename: TypeName;
};

export type DataTypeValueScalar =
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the store.
  | undefined
  // Singular fields:
  | unknown
  | number
  | boolean
  | string
  | null
  // Plural  fields:
  | readonly DataTypeValueScalar[];

export type DataTypeValueLinked =
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the store.
  | undefined
  // Singular fields:
  | null
  | StoreLink
  // Plural fields:
  | readonly DataTypeValueLinked[];

export type StoreRecord = {
  [key: ScalarParentRecordKey]: DataTypeValueScalar;
  [key: LinkedParentRecordKey]: DataTypeValueLinked;
  readonly [key: ParentRecordKey]: DataTypeValueScalar | DataTypeValueLinked;
  // TODO __typename?: T, which is restricted to being a concrete string
  // TODO this shouldn't always be named id
  readonly __typename?: TypeName;
  readonly id?: DataId;
};

export type TypeName = string;
export type DataId = string;

export const ROOT_ID: DataId & '__ROOT' = '__ROOT';

export type StoreLayerData = {
  [index: TypeName]: {
    [index: DataId]: StoreRecord | null;
  } | null;
};

export interface BaseStoreLayerData extends StoreLayerData {
  readonly Query: {
    readonly __ROOT: StoreRecord;
  };
}

const DEFAULT_GC_BUFFER_SIZE = 10;
export function createIsographEnvironmentCore(
  baseStoreLayerData: BaseStoreLayerData,
  networkFunction: IsographNetworkFunction,
  componentFunction: IsographComponentFunction,
  missingFieldHandler?: MissingFieldHandler | null,
  logFunction?: LogFunction | null,
): IsographEnvironment {
  logFunction?.({
    kind: 'EnvironmentCreated',
  });
  let store = {
    kind: 'BaseStoreLayer',
    data: baseStoreLayerData,
    parentStoreLayer: null,
    childStoreLayer: null,
  } as const;
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

export function createIsographStore(): BaseStoreLayerData {
  return {
    Query: {
      [ROOT_ID]: {},
    },
  };
}

export function assertLink(
  link: DataTypeValueLinked,
): StoreLink | null | undefined {
  if (isArray(link)) {
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

export function getLink(maybeLink: DataTypeValueLinked): StoreLink | null {
  if (!isArray(maybeLink)) {
    return maybeLink ?? null;
  }
  return null;
}

export function getOrLoadIsographArtifact(
  environment: IsographEnvironment,
  key: string,
  loader: () => Promise<IsographEntrypoint<any, any, any, any>>,
): PromiseWrapper<IsographEntrypoint<any, any, any, any>> {
  const value = environment.entrypointArtifactCache.get(key);
  if (value != null) {
    return value;
  }
  const wrapped = wrapPromise(loader());
  environment.entrypointArtifactCache.set(key, wrapped);
  return wrapped;
}

export function getOrLoadReaderWithRefetchQueries(
  _environment: IsographEnvironment,
  readerWithRefetchQueries:
    | RawReaderWithRefetchQueries<any, any>
    | ReaderWithRefetchQueriesLoader<any, any>,
): {
  readerWithRefetchQueries: PromiseWrapper<ReaderWithRefetchQueries<any, any>>;
  fieldName: string;
  readerArtifactKind: 'EagerReaderArtifact' | 'ComponentReaderArtifact';
} {
  switch (readerWithRefetchQueries.kind) {
    case 'ReaderWithRefetchQueries':
      const readerArtifact = readerWithRefetchQueries.readerArtifact();
      return {
        readerWithRefetchQueries: wrapResolvedValue({
          kind: 'ReaderWithRefetchQueries',
          nestedRefetchQueries: readerWithRefetchQueries.nestedRefetchQueries,
          readerArtifact: readerArtifact,
        }),
        fieldName: readerArtifact.fieldName,
        readerArtifactKind: readerArtifact.kind,
      };
    case 'ReaderWithRefetchQueriesLoader':
      return {
        // TODO: cache promise wrapper
        readerWithRefetchQueries: wrapPromise(
          readerWithRefetchQueries
            .loader()
            .then((readerWithRefetchQueries) => ({
              kind: 'ReaderWithRefetchQueries',
              nestedRefetchQueries:
                readerWithRefetchQueries.nestedRefetchQueries,
              readerArtifact: readerWithRefetchQueries.readerArtifact(),
            })),
        ),
        fieldName: readerWithRefetchQueries.fieldName,
        readerArtifactKind: readerWithRefetchQueries.readerArtifactKind,
      };
  }
}
