import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { FragmentReference } from './FragmentReference';
import { getOrCreateCachedComponent } from './componentCache';
import { useReadAndSubscribe } from './useReadAndSubscribe';

export function useResult<TReadFromStore extends Object, TClientFieldValue>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
): TClientFieldValue {
  const environment = useIsographEnvironment();

  switch (fragmentReference.readerArtifact.kind) {
    case 'ComponentReaderArtifact': {
      // @ts-expect-error
      return getOrCreateCachedComponent(
        environment,
        fragmentReference.readerArtifact.componentName,
        fragmentReference,
      );
    }
    case 'EagerReaderArtifact': {
      const data = useReadAndSubscribe(environment, fragmentReference);
      return fragmentReference.readerArtifact.resolver(data);
    }
  }
}
