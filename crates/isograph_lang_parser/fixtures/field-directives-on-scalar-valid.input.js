export const BasicField = iso(`
  field Type.Name {
    scalar @loadable
  }
`)();

export const BasicField2 = iso(`
  field Type.Name {
    scalar @loadable(lazyLoadArtifact: false)
  }
`)();

export const BasicField3 = iso(`
  field Type.Name {
    scalar @loadable(lazyLoadArtifact: true)
  }
`)();

export const updatable = iso(`
  field Type.Name {
    scalar @updatable
  }
`)();
