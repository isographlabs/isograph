// Suppress the TypeScript compiler warning for this branded-type trick.
// See discussion: https://github.com/microsoft/TypeScript/issues/202#issuecomment-436900738
// Pattern: “Brand<BaseType, Brand>” leverages TypeScript conditional and `infer` types to create a pseudo-nominal type,
// enabling BaseType to be treated as distinct only when tagged with Brand, even though Brand doesn't exist at runtime.
//
// Explanation:
// - Brand extends `symbol | string` acts as a “brand” identifier.
// - The type uses a conditional check `infer _ extends Brand ? BaseType : never` to strip out BaseType when the branding doesn't match.
// - This yields a branded type system: `Brand<string, "UserId">` is not accidentally assignable to `Brand<string, "ProductId">`.
//
// Usage: Helps enforce semantic distinctions (e.g., distinguishing user IDs from product IDs) even when their runtime values are both strings.
//
// Caveat: This is purely a compile-time trick—Brand is erased in emitted JavaScript, so runtime checks must rely on other mechanisms.
export type Brand<
  BaseType,
  Brand extends symbol | string,
  // @ts-ignore
> = infer _ extends Brand ? BaseType : never;
