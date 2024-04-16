import { useEffect, useState } from 'react';
import { DataId, IsographEnvironment } from './IsographEnvironment';
import { subscribe } from './cache';

export function useRerenderWhenEncounteredRecordChanges(
  environment: IsographEnvironment,
  encounteredRecords: Set<DataId>,
) {
  const [, setState] = useState<object | void>();
  useEffect(() => {
    return subscribe(environment, encounteredRecords, () => {
      return setState({});
    });
    // TODO this is probably buggy — we should re-evaluate the effect when
    // encounteredRecords changes. However, it is not a stable object, so...
    // how?
  }, []);
}
