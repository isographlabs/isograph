type UseImperativeLoadableFieldReturn<TArgs> = {
  loadField: (args: TArgs) => void;
};

// Note: this function doesn't seem to work if there are additional arguments,
// e.g. with set_pet_tagline. Why? This seems to straightforwardly call
// exposedField(args)[1](); Odd.
export function useImperativeExposedMutationField<TArgs>(
  exposedField: (args: TArgs) => [string, () => void],
): UseImperativeLoadableFieldReturn<TArgs> {
  return {
    loadField: (args: TArgs) => {
      const [_id, loader] = exposedField(args);
      loader();
    },
  };
}
