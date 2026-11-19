import type {
  NormalizationSelection,
  NormalizationSelectionSet,
} from '../src/artifacts';
import type { Link, Store, StoreValue } from '../src/store';

/** The root records the fixture's ASTs name, for asserting on results. */
export const queryRoot: Link<'Query'> = { entityName: 'Query', id: '__ROOT' };
export const mutationRoot: Link<'Mutation'> = {
  entityName: 'Mutation',
  id: '__ROOT',
};

/** A new store whose entities, ids, store keys, and values are `records`' keys and values, in order. */
export function storeOf(records: {
  readonly [entityName: string]: {
    readonly [id: string]: { readonly [storeKey: string]: StoreValue };
  };
}): Store {
  return new Map(
    Object.entries(records).map(([entityName, byId]) => [
      entityName,
      new Map(
        Object.entries(byId).map(([id, record]) => [
          id,
          new Map(Object.entries(record)),
        ]),
      ),
    ]),
  );
}

/** The `Selection` reached from `selections` by `responseKeys`, each a response key
 * (`responseKey ?? selectableName`) among the previous selection's target's selections. */
export function selectionAt(
  selections: NormalizationSelectionSet,
  responseKey: string,
  ...rest: ReadonlyArray<string>
): NormalizationSelection {
  const found = selections.find(
    (node): node is NormalizationSelection =>
      node.kind === 'Selection' &&
      (node.responseKey ?? node.selectableName) === responseKey,
  );
  if (found === undefined) {
    throw new Error(`The fixture AST has no selection "${responseKey}".`);
  }
  const [next, ...after] = rest;
  return next === undefined
    ? found
    : selectionAt(found.target?.selections ?? [], next, ...after);
}
