export const BasicField = iso(`
  field Type.Name {
    linked @hello @there {
    }
  }
`)();

export const BasicField2 = iso(`
  field Type.Name {
    linked @loadable(asdf: true) {
    }
  }
`)();

export const BasicField3 = iso(`
  field Type.Name {
    linked @loadable @updatable {
    }
  }
`)();
