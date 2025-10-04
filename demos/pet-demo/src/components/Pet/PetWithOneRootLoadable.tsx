import { iso } from '@iso';
import { Suspense } from 'react';
import { FullPageLoading, type OnlyOneRootLoadableRoute } from '../routes';
import {
  FragmentRenderer,
  useClientSideDefer,
  useLazyReference,
} from '@isograph/react';

export const OnlyOneRootLoadable = iso(`
  field Query.OnlyOneRootLoadablePet(
    $id: ID !
  ) @component {
    PetFavoritePhrase(
      id: $id
    ) @loadable
  }
`)(({ data }) => {
  const { fragmentReference } = useClientSideDefer(data.PetFavoritePhrase);

  return <FragmentRenderer fragmentReference={fragmentReference} />;
});

type OnlyOneRootLoadableRouteProps = {
  route: OnlyOneRootLoadableRoute;
};

// This component demonstrates the problem where we have a root query
// which does not select any fields at first because the only selection
// is @loadable. In a followup PR, we will ensure the empty selection
// set always includes at least `__typename`
export default function OnlyOneRootLoadableRoute({
  route,
}: OnlyOneRootLoadableRouteProps) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.OnlyOneRootLoadablePet`),
    { id: route.id },
    { shouldFetch: 'Yes' },
  );

  return (
    <Suspense fallback={<FullPageLoading />}>
      <FragmentRenderer fragmentReference={fragmentReference} />
    </Suspense>
  );
}
