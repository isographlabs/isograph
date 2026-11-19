import type { NormalizationAst, NormalizationSelectionSet } from './artifacts';
import type { Store, StoreLink, StoreRecord, Variables } from './store';
import { linksIn, storeRecord } from './store';
import { storeKey } from './storeKey';

export type CheckResult =
  | { readonly kind: 'EnoughData' }
  /** `record` lacks a selected value, or is linked to and absent. */
  | { readonly kind: 'MissingData'; readonly record: StoreLink };

/** Whether `store` holds every value `ast` selects from `ast.root` with `variables`. */
export function check(
  store: Store,
  ast: NormalizationAst,
  variables: Variables,
): CheckResult {
  const first = missingRecords(store, ast, variables).next();
  return first.done === true
    ? { kind: 'EnoughData' }
    : { kind: 'MissingData', record: first.value };
}

/** Every missing position, in AST order then list order: the record that lacks a selected
 * value, or the linked record that is absent. */
function* missingRecords(
  store: Store,
  ast: NormalizationAst,
  variables: Variables,
): Generator<StoreLink, void, undefined> {
  const root = storeRecord(store, ast.root);
  if (root === undefined) {
    yield ast.root;
    return;
  }
  yield* missingFrom(store, ast.selections, root, ast.root, variables);
}

function* missingFrom(
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
        if (value === undefined) {
          yield link;
        } else if (node.target != null) {
          for (const next of linksIn(value, link, key)) {
            const nextRecord = storeRecord(store, next);
            if (nextRecord === undefined) {
              yield next;
            } else {
              yield* missingFrom(
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
          yield* missingFrom(store, node.selections, record, link, variables);
        }
        break;
    }
  }
}
