import type { EntityName } from './artifacts';
import { isArray, ownValue } from './util';

/** Every record: `EntityName`, then record id, then the record. */
export type Store = Map<EntityName, Map<RecordId, StoreRecord>>;

/** One record's values by store key. An absent key is missing data; `null` is known data. */
export type StoreRecord = Map<StoreKey, StoreValue>;

export type RecordId = string;
/** A selectable name followed by one chunk per argument, variables substituted. */
export type StoreKey = string;

/** Which of the two a value is, the AST selection under that store key says. */
export type StoreValue = LeafValue | CompositeValue;

/** The response's value for a selection of a leaf entity, as received. */
export type LeafValue = JsonValue;

/** The value of a selection of a composite entity: `null`, a link, or a list of these at any depth. */
export type CompositeValue = null | StoreLink | ReadonlyArray<CompositeValue>;

/** Points at record `id` of the entity `TEntityName`. */
export type Link<TEntityName extends EntityName> = {
  readonly entityName: TEntityName;
  readonly id: RecordId;
};

/** A link to a record of any entity. */
export type StoreLink = Link<EntityName>;

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | ReadonlyArray<JsonValue>
  | JsonObject;
export type JsonObject = { readonly [key: string]: JsonValue };

/** Operation variables by name. Every variable of the operation is present (Variables). */
export type Variables = { readonly [name: string]: JsonValue };

/** The record `link` points at, when the store holds it. */
export function storeRecord(
  store: Store,
  link: StoreLink,
): StoreRecord | undefined {
  return store.get(link.entityName)?.get(link.id);
}

/** Every link in `value`, the value under the composite store key `storeKey` of the record
 * `owner`, in list order at every depth. Throws on a value that is not `null`, a link, or a
 * list of these. */
export function* linksIn(
  value: StoreValue,
  owner: StoreLink,
  storeKey: StoreKey,
): Generator<StoreLink, void, undefined> {
  if (value == null) {
    return;
  }
  if (isArray(value)) {
    for (const item of value) {
      yield* linksIn(item, owner, storeKey);
    }
    return;
  }
  yield linkAt(value, owner, storeKey);
}

/** `value`, a value under the composite store key `storeKey` of the record `owner` that is
 * neither `null` nor a list, as a link. Throws when it is not a link. */
export function linkAt(
  value: Exclude<StoreValue, null | ReadonlyArray<StoreValue>>,
  owner: StoreLink,
  storeKey: StoreKey,
): StoreLink {
  const link = typeof value === 'object' ? linkOf(value) : undefined;
  if (link === undefined) {
    throw new Error(
      `Store record ${owner.entityName}:${owner.id} holds a non-link under the composite store key "${storeKey}"; the store was written with an AST that disagrees with this one.`,
    );
  }
  return link;
}

/** `value` as a link: an object whose own keys are exactly `entityName` and `id`, both strings. */
function linkOf(value: JsonObject): StoreLink | undefined {
  const entityName = ownValue(value, 'entityName');
  const id = ownValue(value, 'id');
  return Object.keys(value).length === 2 &&
    typeof entityName === 'string' &&
    typeof id === 'string'
    ? { entityName, id }
    : undefined;
}
