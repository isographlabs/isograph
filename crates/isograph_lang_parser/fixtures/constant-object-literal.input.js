export const BasicField = iso(`
  field Type.Name($a: Type) {
    scalar(input: { a: $a, b: 12, c: { d: true, e: null } })
  }
`)();
