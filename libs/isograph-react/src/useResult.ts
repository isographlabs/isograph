import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { FragmentReference } from './FragmentReference';
import { getOrCreateCachedComponent } from './componentCache';
import { useReadAndSubscribe } from './useReadAndSubscribe';

export function useResult<TReadFromStore extends Object, TClientFieldValue>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
): TClientFieldValue {
  const environment = useIsographEnvironment();

  switch (fragmentReference.readerArtifact.variant.kind) {
    case 'Component': {
      // @ts-expect-error
      return getOrCreateCachedComponent(
        environment,
        fragmentReference.readerArtifact.variant.componentName,
        fragmentReference,
      );
    }
    case 'Eager': {
      const data = useReadAndSubscribe(environment, fragmentReference);
      // @ts-expect-error resolver is incorrectly typed in ReaderArtifact
      return fragmentReference.readerArtifact.resolver(data);
    }
  }
}
