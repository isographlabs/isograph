export const HomeRoute = iso(`
  unknown Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function Test() {
  return 'Render';
});
