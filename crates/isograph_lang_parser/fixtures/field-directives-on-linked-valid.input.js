export const BasicField = iso(`
  field Type.Name {
    linked @loadable {
    }
  }
`)();

export const updatable = iso(`
  field Type.Name {
    linked @updatable {
    }
  }
`)();
