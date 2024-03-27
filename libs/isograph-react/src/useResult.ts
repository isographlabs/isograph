import { useEffect, useState } from 'react';
import { FragmentReference } from './index';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { subscribe } from './cache';
import { read } from './read';

export function useResult<TReadFromStore extends Object, TResolverResult>(
  fragmentReference: FragmentReference<TReadFromStore, TResolverResult>,
): TResolverResult {
  const environment = useIsographEnvironment();

  const [, setState] = useState<object | void>();
  useEffect(() => {
    return subscribe(environment, () => {
      return setState({});
    });
  }, []);

  return read(environment, fragmentReference);
}
