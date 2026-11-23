export function assertNever(value: never): never {
  throw new Error(`Unhandled variant: ${String(value)}`);
}
