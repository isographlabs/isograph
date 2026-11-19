import type {
  EntityName,
  ReaderArtifact,
  ResolverParam,
  SelectionSetData,
} from './artifacts';
import type { ReadMissingData, ReadOk } from './read';
import type { StoreLink, Variables } from './store';

/** What a subscription delivers: the reader's data, `MissingData`, or what the read threw. */
export type SubscriptionResult =
  | ReadOk<SelectionSetData>
  | ReadMissingData
  | ReadErr;

/** A read that threw: a nested resolver threw, or a store invariant broke (store.md, Errors). */
export type ReadErr = {
  readonly kind: 'Err';
  readonly error: unknown;
};

export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The result last read: at subscribe, or the last one the callback received. */
  delivered: SubscriptionResult;
  readonly callback: (result: SubscriptionResult) => void;
  /** Called after each change to `delivered`, after `callback`. */
  readonly listeners: Set<() => void>;
};
