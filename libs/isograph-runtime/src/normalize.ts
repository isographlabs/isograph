import type {
  NormalizationAst,
  NormalizationCompositeTarget,
  NormalizationSelection,
  NormalizationSelectionSet,
} from './artifacts';
import { recordLink } from './recordId';
import type {
  CompositeValue,
  JsonObject,
  JsonValue,
  Store,
  StoreLink,
  StoreRecord,
  Variables,
} from './store';
import { storeKey } from './storeKey';
import { getOrInsert, isArray, ownValue } from './util';

/** What every step of one `normalize` call reads besides its position. */
type Normalization = {
  readonly store: Store;
  readonly discriminatorResponseKey: string;
  readonly variables: Variables;
};

type ResponsePath = ReadonlyArray<string | number>;

/** Writes `response` into `store` from `ast.root` down, by `ast` and `variables`. Throws on a
 * response that does not match `ast` (Errors). */
export function normalize(
  store: Store,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void {
  const normalization: Normalization = {
    store,
    discriminatorResponseKey: ast.discriminatorResponseKey,
    variables,
  };
  writeSelections(
    normalization,
    ast.selections,
    response,
    recordAt(store, ast.root),
    ast.root,
    [],
  );
}

function writeSelections(
  normalization: Normalization,
  selections: NormalizationSelectionSet,
  response: JsonObject,
  record: StoreRecord,
  link: StoreLink,
  path: ResponsePath,
): void {
  selections.forEach((node) => {
    switch (node.kind) {
      case 'Selection':
        writeSelection(normalization, node, response, record, link, path);
        return;
      case 'Conditional':
        if (node.condition.entityName === link.entityName) {
          writeSelections(
            normalization,
            node.selections,
            response,
            record,
            link,
            path,
          );
        }
        return;
    }
  });
}

function writeSelection(
  normalization: Normalization,
  selection: NormalizationSelection,
  response: JsonObject,
  record: StoreRecord,
  link: StoreLink,
  path: ResponsePath,
): void {
  const responseKey = selection.responseKey ?? selection.selectableName;
  const value = ownValue(response, responseKey);
  if (value === undefined) {
    throw new Error(
      `Malformed response at ${JSON.stringify(path)}: expected the response key "${responseKey}".`,
    );
  }
  record.set(
    storeKey(selection, normalization.variables),
    selection.target == null
      ? value
      : compositeValue(
          normalization,
          selection,
          selection.target,
          value,
          link,
          [],
          [...path, responseKey],
        ),
  );
}

/** `value`, the response's value of the composite `selection` or one of its list items, with
 * each object written to its record and replaced by the link to it. */
function compositeValue(
  normalization: Normalization,
  selection: NormalizationSelection,
  target: NormalizationCompositeTarget,
  value: JsonValue,
  parent: StoreLink,
  listIndexes: ReadonlyArray<number>,
  path: ResponsePath,
): CompositeValue {
  if (value == null) {
    return null;
  }
  if (isArray(value)) {
    return value.map((item, index) =>
      compositeValue(
        normalization,
        selection,
        target,
        item,
        parent,
        [...listIndexes, index],
        [...path, index],
      ),
    );
  }
  if (typeof value !== 'object') {
    throw new Error(
      `Malformed response at ${JSON.stringify(path)}: expected an object, a list, or null for "${selection.responseKey ?? selection.selectableName}", got ${JSON.stringify(value)}.`,
    );
  }
  const link = recordLink(
    value,
    target,
    normalization.discriminatorResponseKey,
    { parent, selection, listIndexes, path },
    normalization.variables,
  );
  writeSelections(
    normalization,
    target.selections,
    value,
    recordAt(normalization.store, link),
    link,
    path,
  );
  return link;
}

/** The store's record at `link`, inserting an empty one when the store has none. */
function recordAt(store: Store, link: StoreLink): StoreRecord {
  return getOrInsert(
    getOrInsert(store, link.entityName, () => new Map()),
    link.id,
    () => new Map(),
  );
}
