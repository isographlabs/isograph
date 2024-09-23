import type { ReaderAst, ReaderLinkedField, ReaderScalarField } from './reader';
export function mergeUsingReaderAst(
  field: ReaderScalarField | ReaderLinkedField,
  oldItem: unknown,
  newItem: unknown,
): unknown {
  if (newItem === null) {
    return oldItem === null ? oldItem : newItem;
  }

  if (newItem === undefined) {
    return oldItem === undefined ? oldItem : newItem;
  }

  if (Array.isArray(newItem)) {
    if (!Array.isArray(oldItem)) {
      return newItem;
    }

    return mergeArraysUsingReaderAst(field, oldItem, newItem);
  }

  switch (field.kind) {
    case 'Scalar':
      return oldItem === newItem ? oldItem : newItem;
    case 'Linked':
      if (oldItem == null) {
        return newItem;
      }

      return mergeObjectsUsingReaderAst(field.selections, oldItem, newItem);
    default: {
      // Ensure we have covered all variants
      let _: never = field;
      _;
      throw new Error('Unexpected case.');
    }
  }
}

export function mergeArraysUsingReaderAst(
  field: ReaderScalarField | ReaderLinkedField,
  oldItems: ReadonlyArray<unknown>,
  newItems: Array<unknown>,
): ReadonlyArray<unknown> {
  if (newItems.length !== oldItems.length) {
    return newItems;
  }

  let canRecycle = true;
  for (let i = 0; i < newItems.length; i++) {
    const mergedItem = mergeUsingReaderAst(field, oldItems[i], newItems[i]);
    if (mergedItem !== oldItems[i]) {
      canRecycle = false;
    } else {
      newItems[i] = mergedItem;
    }
  }

  return canRecycle ? oldItems : newItems;
}

export function mergeObjectsUsingReaderAst(
  ast: ReaderAst<object>,
  oldItemObject: object,
  newItemObject: object,
): object {
  let canRecycle = true;
  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar':
      case 'Linked':
        const key = field.alias ?? field.fieldName;
        // @ts-expect-error
        const oldValue = oldItemObject[key];
        // @ts-expect-error
        const newValue = newItemObject[key];

        const mergedValue = mergeUsingReaderAst(field, oldValue, newValue);
        if (mergedValue !== oldValue) {
          canRecycle = false;
        } else {
          // @ts-expect-error
          newItemObject[key] = mergedValue;
        }
        break;
      case 'Resolver': {
        const key = field.alias;
        // @ts-expect-error
        const oldValue = oldItemObject[key];
        // @ts-expect-error
        const newValue = newItemObject[key];

        if (oldValue !== newValue) {
          canRecycle = false;
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

  return canRecycle ? oldItemObject : newItemObject;
}
