import type { StoreLink } from './IsographEnvironment';
import type { ReaderAst, ReaderLinkedField, ReaderScalarField } from './reader';

function mergeUsingReaderAst(
  field: ReaderScalarField | ReaderLinkedField,
  oldItem: unknown,
  newItem: unknown,
): unknown {
  if (newItem == null || oldItem == undefined) {
    return newItem;
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

function mergeArraysUsingReaderAst(
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
        if (field.kind === 'Linked' && field.refetchQueryIndex != null) {
          // client pointers are functions, so we can't merge them
          canRecycle = false;
          break;
        }
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
      case 'Link': {
        const key = field.alias;
        // @ts-expect-error
        const oldValue: StoreLink = oldItemObject[key];
        // @ts-expect-error
        const newValue: StoreLink = newItemObject[key];

        if (
          oldValue.__link !== newValue.__link ||
          oldValue.__typename !== newValue.__typename
        ) {
          canRecycle = false;
        } else {
          // @ts-expect-error
          newItemObject[key] = oldValue;
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

export function mergeArrays<T extends unknown[]>(oldArray: T, newArray: T): T {
  if (newArray.length !== oldArray.length) {
    return newArray;
  }

  let canRecycle = true;
  for (let i = 0; i < newArray.length; i++) {
    const mergedItem = mergeValues(oldArray[i], newArray[i]);
    if (mergedItem !== oldArray[i]) {
      canRecycle = false;
    } else {
      newArray[i] = mergedItem;
    }
  }

  return canRecycle ? oldArray : newArray;
}

function mergeValues<T>(oldObject: T, newObject: T): T {
  if (newObject == null || oldObject == null) {
    return newObject;
  }

  if (Array.isArray(newObject)) {
    if (!Array.isArray(oldObject)) {
      return newObject;
    }
    return mergeArrays(oldObject, newObject);
  }

  if (typeof newObject === 'object') {
    if (typeof oldObject !== 'object') {
      return newObject;
    }

    if (Object.keys(oldObject).length !== Object.keys(newObject).length) {
      return newObject;
    }

    let canRecycle = true;

    for (const key of Object.keys(newObject)) {
      // @ts-expect-error
      const mergedValue = mergeValues(oldObject[key], newObject[key]);
      // @ts-expect-error
      if (mergedValue !== oldObject[key]) {
        canRecycle = false;
      } else {
        // @ts-expect-error
        newObject[key] = mergedValue;
      }
    }

    return canRecycle ? oldObject : newObject;
  }

  return newObject;
}
