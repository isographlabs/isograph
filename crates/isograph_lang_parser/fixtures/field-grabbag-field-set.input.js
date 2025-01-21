export const BasicField = iso(`
  field Type.Name
  {
    scalar
    linked {
      scalarWithComma,
    }
    linkedWithComma {
      blah
    },
    multiple, on, the, same, line
    including, youCanEndWithALinkedField {
      butWhyWouldYouDoThat
    }
  }
`)();
