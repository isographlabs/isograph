export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRouteComponent({ data }) {
  const { fragmentReference, loadFragmentReference } = useImperativeReference(
    iso(`entrypoint Query.PetFavoritePhrase`),
  );
  
  return "Render";
});
