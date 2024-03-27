import { useEffect, useState } from 'react';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { subscribe } from './cache';
import { read } from './read';
import { FragmentReference } from './FragmentReference';

export function useResult<TReadFromStore extends Object, TClientFieldValue>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
): TClientFieldValue {
  const environment = useIsographEnvironment();

  const [, setState] = useState<object | void>();
  useEffect(() => {
    return subscribe(environment, () => {
      return setState({});
    });
  }, []);

  return read(environment, fragmentReference);
}
