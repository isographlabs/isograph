export function areEqualWithDeepComparison(
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

    return areEqualArraysWithDeepComparison(oldItem, newItem);
  }

  if (typeof newItem === 'object') {
    if (typeof oldItem !== 'object') {
      return false;
    }

    if (oldItem === null) {
      return false;
    }

    return areEqualObjectsWithDeepComparison(oldItem, newItem);
  }

  return newItem === oldItem;
}

export function areEqualArraysWithDeepComparison(
  oldItems: ReadonlyArray<unknown>,
  newItems: ReadonlyArray<unknown>,
): boolean {
  if (newItems.length !== oldItems.length) {
    return false;
  }

  for (let i = 0; i < newItems.length; i++) {
    if (!areEqualWithDeepComparison(oldItems[i], newItems[i])) {
      return false;
    }
  }

  return true;
}

export function areEqualObjectsWithDeepComparison(
  oldItemObject: object,
  newItemObject: object,
): boolean {
  const oldKeys = Object.keys(oldItemObject);
  const newKeys = Object.keys(newItemObject);

  if (oldKeys.length !== newKeys.length) {
    return false;
  }

  for (const oldKey of oldKeys) {
    if (!(oldKey in newItemObject)) {
      return false;
    }
    // @ts-expect-error
    const oldValue = oldItemObject[oldKey];
    // @ts-expect-error
    const newValue = newItemObject[oldKey];

    if (!areEqualWithDeepComparison(oldValue, newValue)) {
      return false;
    }
  }
  return true;
}
