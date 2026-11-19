import type {
  EntityName,
  NormalizationAst,
  NormalizationSelectionSet,
} from './artifacts';
import type {
  RecordId,
  Store,
  StoreLink,
  StoreRecord,
  Variables,
} from './store';
import { linksIn, storeRecord } from './store';
import { storeKey } from './storeKey';
import { getOrInsert } from './util';

/** What the caller keeps: an AST and its variables. The walk starts at `ast.root`. */
export type RetainedQuery = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
};

/** Deletes from `store` every record no query in `retained` reaches. */
export function garbageCollect(
  store: Store,
  retained: ReadonlyArray<RetainedQuery>,
): void {
  const reachable = new Map<EntityName, Set<RecordId>>();
  for (const link of reachableRecords(store, retained)) {
    getOrInsert(reachable, link.entityName, () => new Set()).add(link.id);
  }
  store.forEach((records, entityName) => {
    const kept = reachable.get(entityName);
    records.forEach((_record, id) => {
      if (kept?.has(id) !== true) {
        records.delete(id);
      }
    });
    if (records.size === 0) {
      store.delete(entityName);
    }
  });
}

/** Every record a query in `retained` reaches, once per path. */
function* reachableRecords(
  store: Store,
  retained: ReadonlyArray<RetainedQuery>,
): Generator<StoreLink, void, undefined> {
  for (const { ast, variables } of retained) {
    const root = storeRecord(store, ast.root);
    if (root !== undefined) {
      yield ast.root;
      yield* reachableFrom(store, ast.selections, root, ast.root, variables);
    }
  }
}

function* reachableFrom(
  store: Store,
  selections: NormalizationSelectionSet,
  record: StoreRecord,
  link: StoreLink,
  variables: Variables,
): Generator<StoreLink, void, undefined> {
  for (const node of selections) {
    switch (node.kind) {
      case 'Selection': {
        const key = storeKey(node, variables);
        const value = record.get(key);
        if (node.target != null && value !== undefined) {
          for (const next of linksIn(value, link, key)) {
            const nextRecord = storeRecord(store, next);
            if (nextRecord !== undefined) {
              yield next;
              yield* reachableFrom(
                store,
                node.target.selections,
                nextRecord,
                next,
                variables,
              );
            }
          }
        }
        break;
      }
      case 'Conditional':
        if (node.condition.entityName === link.entityName) {
          yield* reachableFrom(store, node.selections, record, link, variables);
        }
        break;
    }
  }
}
