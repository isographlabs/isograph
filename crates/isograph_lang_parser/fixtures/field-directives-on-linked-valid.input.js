export const updatable = iso(`
  field Type.Name {
    linked @updatable {
    }
  }
`)();
