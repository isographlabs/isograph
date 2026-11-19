import type {
  ReaderSelection,
  ReaderSelectionSet,
  SelectionSetData,
} from './artifacts';
import { isArray } from './util';

/** Whether two reads of `selections` hold equal values, upstream's rule. */
export function selectionSetDataAreEqual(
  selections: ReaderSelectionSet,
  delivered: SelectionSetData,
  read: SelectionSetData,
): boolean {
  return selections.every((selection) =>
    selectionValuesAreEqual(
      selection,
      delivered[selection.alias],
      read[selection.alias],
    ),
  );
}

function selectionValuesAreEqual(
  selection: ReaderSelection,
  delivered: unknown,
  read: unknown,
): boolean {
  switch (selection.kind) {
    case 'Scalar':
      return valuesAreEqual(delivered, read, (a, b) => a === b);
    case 'Linked':
    case 'Refinement':
      return valuesAreEqual(
        delivered,
        read,
        (a, b) =>
          isSelectionSetData(a) &&
          isSelectionSetData(b) &&
          selectionSetDataAreEqual(selection.selections, a, b),
      );
    case 'Resolver':
      return delivered === read;
  }
}

/** `null` equals only `null`; lists by length and item, at every depth; other values by
 * `itemsAreEqual`. */
function valuesAreEqual(
  delivered: unknown,
  read: unknown,
  itemsAreEqual: (delivered: unknown, read: unknown) => boolean,
): boolean {
  if (delivered == null || read == null) {
    return delivered === read;
  }
  if (isArray(delivered) || isArray(read)) {
    return (
      isArray(delivered) &&
      isArray(read) &&
      delivered.length === read.length &&
      delivered.every((item, index) =>
        valuesAreEqual(item, read[index], itemsAreEqual),
      )
    );
  }
  return itemsAreEqual(delivered, read);
}

function isSelectionSetData(value: unknown): value is SelectionSetData {
  return typeof value === 'object' && value != null && !isArray(value);
}
