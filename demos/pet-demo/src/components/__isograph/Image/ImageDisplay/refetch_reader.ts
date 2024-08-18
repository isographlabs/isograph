import type { RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
const includeReadOutData = (variables: any, readOutData: any) => {
  variables.id = readOutData.id;
  return variables;
};

import { makeNetworkRequest, wrapResolvedValue, type IsographEnvironment, type FragmentReference, type RefetchQueryNormalizationArtifactWrapper, type DataId, type TopLevelReaderArtifact } from '@isograph/react';
import { type ItemCleanupPair } from '@isograph/react-disposable-state';
const resolver = (
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact,
  readOutData: any,
  filteredVariables: any,
  rootId: DataId,
  // TODO type this
  readerArtifact: TopLevelReaderArtifact<any, any, any> | null,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
) => (): ItemCleanupPair<FragmentReference<any, any>> | undefined => {
  const variables = includeReadOutData(filteredVariables, readOutData);
  const [networkRequest, disposeNetworkRequest] = makeNetworkRequest(environment, artifact, variables);
  if (readerArtifact == null) return;
  const fragmentReference = {
    kind: "FragmentReference",
    readerWithRefetchQueries: wrapResolvedValue({
      kind: "ReaderWithRefetchQueries",
      readerArtifact,
      nestedRefetchQueries,
    } as const),
    root: rootId,
    variables,
    networkRequest,
  } as const;
  return [fragmentReference, disposeNetworkRequest];
};


const readerAst: ReaderAst<unknown> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

const artifact: RefetchReaderArtifact = {
  kind: "RefetchReaderArtifact",
  // @ts-ignore
  resolver,
  readerAst,
};

export default artifact;
