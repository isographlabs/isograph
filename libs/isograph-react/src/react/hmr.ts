/**
 * Babel replaces iso(`...`)(Field) with hmr(Field) to enable HMR in React components.
 * Components can take only one parameter but isograph fields take two parameters:
 * - firstParameter: the data and parameters for the component
 * - additionalRuntimeProps: any additional props passed to the component at runtime
 *
 * This function adapts the isograph field signature to the standard React component signature.
 *
 * For HMR to work correctly, this has to be done at the top level of the module so that component is stable.
 */
export function hmr<T, U, R>(
  clientFieldResolver: (firstParameter: T, additionalRuntimeProps: U) => R,
): (props: { firstParameter: T; additionalRuntimeProps: U }) => R {
  return (props) =>
    clientFieldResolver(props.firstParameter, props.additionalRuntimeProps);
}
