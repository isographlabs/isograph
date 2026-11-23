/**
 * Babel replaces iso(`...`)(Field) with hmr(Field) to enable HMR in React components.
 * Components can take only one parameter but isograph fields take two parameters:
 * - firstParameter: the data and parameters for the component
 * - additionalRuntimeProps: any additional props passed to the component at runtime
 *
 * This function adapts the isograph field signature to the standard React component signature.
 *
 * For HMR to work correctly this has to be done:
 * 1. at the top level of the module so that component is stable,
 * 2. before HMR babel transform runs.
 */
export function hmr<TReadFromStore, U, TClientFieldValue>(
  clientFieldResolver: (
    firstParameter: TReadFromStore,
    additionalRuntimeProps: U,
  ) => TClientFieldValue,
) {
  function Component(props: {
    firstParameter: TReadFromStore;
    additionalRuntimeProps: U;
  }): TClientFieldValue {
    return clientFieldResolver(
      props.firstParameter,
      props.additionalRuntimeProps,
    );
  }
  Component.displayName = clientFieldResolver.name;
  return Component;
}
