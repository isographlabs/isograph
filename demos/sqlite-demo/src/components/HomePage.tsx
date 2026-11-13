import { iso } from '@iso';

export const HomePage = iso(`
  field planets.HomePage @component {
    planetName: name
    id
    climate
    surface_water
    orbital_period
  }
`)(function HomePageComponent({ data }) {
  return <div>Home Page - {data.planetName}</div>;
});
