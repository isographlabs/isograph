import { iso } from '@iso';
import { Suspense } from 'react';
import { FullPageLoading, OnlyOneRootLoadableRoute, Route } from '../routes';
import { FragmentRenderer, useLazyReference } from '@isograph/react';

export const OnlyOneRootLoadable = iso(`
  field Query.OnlyOneRootLoadablePet(
    $id: ID !
  ) @component {
    PetDetailRoute(
      id: $id
    ) @loadable
  }
`)(({ data }) => {
  return <data.PetDetailRoute />;
});

type OnlyOneRootLoadablePageProps = {
  route: OnlyOneRootLoadableRoute;
};

export default function OnlyOneRootLoadablePage({
  route,
}: OnlyOneRootLoadablePageProps) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.OnlyOneRootLoadablePet`),
    { id: route.id },
  );

  return (
    <Suspense fallback={<FullPageLoading />}>
      <FragmentRenderer fragmentReference={fragmentReference} />
    </Suspense>
  );
}
