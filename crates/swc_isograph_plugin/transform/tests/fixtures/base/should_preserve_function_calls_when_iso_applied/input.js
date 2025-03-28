export const HomeRoute = iso(`
    field Query.HomeRoute @component {
      pets {
        id
      }
    }
  `)(function HomeRouteComponent() {
  return 'Render';
});
