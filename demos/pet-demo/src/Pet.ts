const iso: any = null;

export const fullName = iso(`
    field Pet.fullName @foo(bar: 123, baz: $qux)
    {
        id
    }
`);
