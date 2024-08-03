import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { FragmentReference } from '../core/FragmentReference';
import { getOrCreateCachedComponent } from '../core/componentCache';
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
