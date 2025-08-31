// Suppress the TypeScript compiler warning for this branded-type trick.
// See discussion: https://github.com/microsoft/TypeScript/issues/202#issuecomment-436900738
// Pattern: “Brand<A, B>” leverages TypeScript conditional and `infer` types to create a pseudo-nominal type,
// enabling A to be treated as distinct only when tagged with B, even though B doesn't exist at runtime.
//
// Explanation:
// - B extends `symbol | string` acts as a “brand” identifier.
// - The type uses a conditional check `infer _ extends B ? A : never` to strip out A when the branding doesn't match.
// - This yields a branded type system: `Brand<string, "UserId">` is not accidentally assignable to `Brand<string, "ProductId">`.
//
// Usage: Helps enforce semantic distinctions (e.g., distinguishing user IDs from product IDs) even when their runtime values are both strings.
//
// Caveat: This is purely a compile-time trick—B is erased in emitted JavaScript, so runtime checks must rely on other mechanisms.
// @ts-ignore
export type Brand<A, B extends symbol | string> = infer _ extends B ? A : never;
