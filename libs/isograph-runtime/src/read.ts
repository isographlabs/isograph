import type {
  Arguments,
  ArgumentValue,
  EntityName,
  ReaderArtifact,
  ReaderSelection,
  ReaderSelectionSet,
  ResolverParam,
  SelectionSetData,
} from './artifacts';
import {
  linkAt,
  storeRecord,
  type JsonValue,
  type Link,
  type Store,
  type StoreKey,
  type StoreLink,
  type StoreValue,
  type Variables,
} from './store';
import { storeKey } from './storeKey';
import { isArray, ownValue } from './util';

export type ReadResult<T> = ReadOk<T> | ReadMissingData;

export type ReadOk<T> = { readonly kind: 'Ok'; readonly value: T };

/** A value the read needs is not in the store. */
export type ReadMissingData = {
  readonly kind: 'MissingData';
  /** What was absent and where, for a person reading it. */
  readonly reason: string;
  /** The `MissingData` of the nested read that ended this one. */
  readonly nestedReason?: ReadMissingData;
  /** The record that lacks the value, or that a node read a value from and is absent. */
  readonly recordLink: StoreLink;
};

/**
 * The value `artifact`'s resolver returns for the record `root`, with `variables` as the
 * field's parameters, or `MissingData` when the store lacks a value its reader AST reads.
 * `root`'s entity is `artifact`'s parent entity.
 */
export function read<TEntityName extends EntityName, TOutput>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ReadResult<TOutput> {
  if (storeRecord(store, root) === undefined) {
    return {
      kind: 'MissingData',
      reason: `No record for ${recordName(root)}`,
      recordLink: root,
    };
  }
  const data = readSelectionSet(store, artifact.readerAst, root, variables);
  if (data.kind === 'MissingData') {
    return data;
  }
  return ok(
    artifact.resolver({ data: data.value, parameters: variables, link: root }),
  );
}

function ok<T>(value: T): ReadOk<T> {
  return { kind: 'Ok', value };
}

function recordName(record: StoreLink): string {
  return `${record.entityName}:${record.id}`;
}

/** `missing` as the cause of a missing value for `name` on `root`. */
function missingDataFor(
  name: string,
  root: StoreLink,
  missing: ReadMissingData,
): ReadMissingData {
  return {
    kind: 'MissingData',
    reason: `Missing data for ${name} on ${recordName(root)}`,
    nestedReason: missing,
    recordLink: missing.recordLink,
  };
}

/** Each node's value under its alias; the first `MissingData` ends the read. */
function readSelectionSet(
  store: Store,
  selections: ReaderSelectionSet,
  root: StoreLink,
  variables: Variables,
): ReadResult<SelectionSetData> {
  const values = allOk(selections, (selection) =>
    readSelection(store, selection, root, variables),
  );
  if (values.kind === 'MissingData') {
    return values;
  }
  return ok(
    Object.fromEntries(
      selections.map((selection, index) => [
        selection.alias,
        values.value[index],
      ]),
    ),
  );
}

function readSelection(
  store: Store,
  selection: ReaderSelection,
  root: StoreLink,
  variables: Variables,
): ReadResult<unknown> {
  switch (selection.kind) {
    case 'Scalar':
      return readStoreValue(store, root, storeKey(selection, variables));
    case 'Linked': {
      const key = storeKey(selection, variables);
      const value = readStoreValue(store, root, key);
      if (value.kind === 'MissingData') {
        return value;
      }
      const linked = readLinkedValue(
        store,
        value.value,
        selection.selections,
        root,
        key,
        variables,
      );
      return linked.kind === 'MissingData'
        ? missingDataFor(key, root, linked)
        : linked;
    }
    case 'Refinement': {
      if (selection.condition.entityName !== root.entityName) {
        return ok(null);
      }
      const refined = readSelectionSet(
        store,
        selection.selections,
        root,
        variables,
      );
      return refined.kind === 'MissingData'
        ? missingDataFor(selection.alias, root, refined)
        : refined;
    }
    case 'Resolver': {
      const resolved = read(
        store,
        selection.readerArtifact(),
        root,
        childVariables(variables, selection.arguments),
      );
      return resolved.kind === 'MissingData'
        ? missingDataFor(selection.alias, root, resolved)
        : resolved;
    }
  }
}

/** The value stored on `root`'s record under `key`. */
function readStoreValue(
  store: Store,
  root: StoreLink,
  key: StoreKey,
): ReadResult<StoreValue> {
  const record = storeRecord(store, root);
  if (record === undefined) {
    return {
      kind: 'MissingData',
      reason: `No record for ${recordName(root)}`,
      recordLink: root,
    };
  }
  const value = record.get(key);
  if (value === undefined) {
    return {
      kind: 'MissingData',
      reason: `No value for ${key} on ${recordName(root)}`,
      recordLink: root,
    };
  }
  return ok(value);
}

/**
 * `null` is `null`; a list maps element by element at any depth; a link reads `selections` at
 * its record. `owner` and `key` locate the value, for the broken-invariant message.
 */
function readLinkedValue(
  store: Store,
  value: StoreValue,
  selections: ReaderSelectionSet,
  owner: StoreLink,
  key: StoreKey,
  variables: Variables,
): ReadResult<unknown> {
  if (value == null) {
    return ok(null);
  }
  if (isArray(value)) {
    return allOk(value, (item) =>
      readLinkedValue(store, item, selections, owner, key, variables),
    );
  }
  return readSelectionSet(
    store,
    selections,
    linkAt(value, owner, key),
    variables,
  );
}

/** A `Resolver` node's variables: each argument with the reader's variables substituted. */
function childVariables(
  variables: Variables,
  args: Arguments | null,
): Variables {
  return Object.fromEntries(
    (args ?? []).flatMap(([name, value]) => {
      const substituted = argumentValue(variables, value);
      return substituted === undefined ? [] : [[name, substituted]];
    }),
  );
}

/** `undefined` when `value` is a variable absent from `variables`. */
function argumentValue(
  variables: Variables,
  value: ArgumentValue,
): JsonValue | undefined {
  switch (value.kind) {
    case 'Variable':
      return ownValue(variables, value.name);
    case 'Literal':
    case 'String':
      return value.value;
    case 'Object':
      return Object.fromEntries(
        value.value.map(([name, entry]) => [
          name,
          argumentValue(variables, entry) ?? null,
        ]),
      );
  }
}

/**
 * `f` of every item, or the first `MissingData`. A loop: JavaScript has no short-circuiting
 * collect over results.
 */
function allOk<T, U>(
  items: ReadonlyArray<T>,
  f: (item: T) => ReadResult<U>,
): ReadResult<ReadonlyArray<U>> {
  const values: U[] = [];
  for (const item of items) {
    const result = f(item);
    if (result.kind === 'MissingData') {
      return result;
    }
    values.push(result.value);
  }
  return ok(values);
}
