import { iso } from '@iso';
import { useLazyReference, useResult } from '@isograph/react';

export default function HomePageRoute() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint planets.HomePage`),
    {},
  );
  const HomePage = useResult(fragmentReference);
  return <HomePage />;
}
