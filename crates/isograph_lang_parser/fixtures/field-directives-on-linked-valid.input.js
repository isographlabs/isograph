export const BasicField = iso(`
  field Type.Name {
    linked @loadable {
    }
  }
`)();

export const BasicField2 = iso(`
  field Type.Name {
    linked @loadable(lazyLoadArtifact: true) {
    }
  }
`)();

export const BasicField3 = iso(`
  field Type.Name {
    linked @loadable(lazyLoadArtifact: false) {
    }
  }
`)();

export const updatable = iso(`
  field Type.Name {
    linked @updatable {
    }
  }
`)();
