import type { ReaderAst, ReaderLinkedField, ReaderScalarField } from './reader';
export function areEqualWithDeepComparison(
  field: ReaderScalarField | ReaderLinkedField,
  oldItem: unknown,
  newItem: unknown,
): boolean {
  if (newItem === null) {
    return oldItem === null;
  }

  if (newItem === undefined) {
    return oldItem === undefined;
  }

  if (Array.isArray(newItem)) {
    if (!Array.isArray(oldItem)) {
      return false;
    }

    return areEqualArraysWithDeepComparison(field, oldItem, newItem);
  }

  switch (field.kind) {
    case 'Scalar':
      return newItem === oldItem;
    case 'Linked':
      if (oldItem == null) {
        return false;
      }

      return areEqualObjectsWithDeepComparison(
        field.selections,
        oldItem,
        newItem,
      );
    default: {
      // Ensure we have covered all variants
      let _: never = field;
      _;
      throw new Error('Unexpected case.');
    }
  }
}

export function areEqualArraysWithDeepComparison(
  field: ReaderScalarField | ReaderLinkedField,
  oldItems: ReadonlyArray<unknown>,
  newItems: ReadonlyArray<unknown>,
): boolean {
  if (newItems.length !== oldItems.length) {
    return false;
  }

  for (let i = 0; i < newItems.length; i++) {
    if (!areEqualWithDeepComparison(field, oldItems[i], newItems[i])) {
      return false;
    }
  }

  return true;
}

export function areEqualObjectsWithDeepComparison(
  ast: ReaderAst<object>,
  oldItemObject: object,
  newItemObject: object,
): boolean {
  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar':
      case 'Linked':
        const key = field.alias ?? field.fieldName;
        // @ts-expect-error
        const oldValue = oldItemObject[key];
        // @ts-expect-error
        const newValue = newItemObject[key];
        if (!areEqualWithDeepComparison(field, oldValue, newValue)) {
          return false;
        }
        break;
      case 'Resolver': {
        const key = field.alias;
        // @ts-expect-error
        const oldValue = oldItemObject[key];
        // @ts-expect-error
        const newValue = newItemObject[key];

        if (oldValue !== newValue) {
          return false;
        }
        break;
      }
      case 'ImperativelyLoadedField':
      case 'LoadablySelectedField':
        break;
      default: {
        // Ensure we have covered all variants
        let _: never = field;
        _;
        throw new Error('Unexpected case.');
      }
    }
  }

  return true;
}
