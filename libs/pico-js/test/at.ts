/** The element of `array` at `index`. Throws when `index` is out of bounds. */
export function at<T>(array: ReadonlyArray<T>, index: number): T {
  const element = array[index];
  if (element === undefined) {
    throw new Error(`index ${index} is out of bounds`);
  }
  return element;
}
