import type {
  Arguments,
  ArgumentValue,
  NormalizationSelection,
} from './artifacts';
import type { JsonValue, StoreKey, Variables } from './store';
import { ownValue, stableCopy } from './util';

const FIRST_SPLIT_KEY = '____';
const SECOND_SPLIT_KEY = '___';

/** `selection.selectableName`, then `argumentChunks(selection.arguments, variables)`. */
export function storeKey(
  selection: NormalizationSelection,
  variables: Variables,
): StoreKey {
  return (
    selection.selectableName + argumentChunks(selection.arguments, variables)
  );
}

/** One `____<name>___<value>` chunk per argument, in the AST's order (sorted by name), with
 * `<value>` the argument's value as `JSON.stringify(stableCopy(value))` writes it. */
export function argumentChunks(
  args: Arguments | null,
  variables: Variables,
): string {
  return (args ?? [])
    .map(
      ([name, value]) =>
        `${FIRST_SPLIT_KEY}${name}${SECOND_SPLIT_KEY}${JSON.stringify(stableCopy(argumentValue(value, variables)))}`,
    )
    .join('');
}

function argumentValue(value: ArgumentValue, variables: Variables): JsonValue {
  switch (value.kind) {
    case 'Literal':
      return value.value;
    case 'String':
      return value.value;
    case 'Variable':
      return variableValue(variables, value.name);
    case 'Object':
      return Object.fromEntries(
        value.value.map(([name, item]) => [
          name,
          argumentValue(item, variables),
        ]),
      );
  }
}

function variableValue(variables: Variables, name: string): JsonValue {
  const value = ownValue(variables, name);
  if (value === undefined) {
    throw new Error(
      `Missing variable "${name}": every variable of the operation is required.`,
    );
  }
  return value;
}
