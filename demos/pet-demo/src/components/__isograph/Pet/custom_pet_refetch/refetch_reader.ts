import type { RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
const includeReadOutData = (variables: any, readOutData: any) => {
  return variables;
};

import { makeNetworkRequest, wrapResolvedValue, type IsographEnvironment, type Link, type TopLevelReaderArtifact, type FragmentReference, type RefetchQueryNormalizationArtifactWrapper } from '@isograph/react';
import { type ItemCleanupPair } from '@isograph/react-disposable-state';
const resolver = (
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact,
  readOutData: any,
  filteredVariables: any,
  rootLink: Link,
  // If readerArtifact is null, the return value is undefined.
  // TODO reflect this in the types.
  readerArtifact: TopLevelReaderArtifact<any, any, any> | null,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
) => (): ItemCleanupPair<FragmentReference<any, any>> | undefined => {
  const variables = includeReadOutData(filteredVariables, readOutData);
  const [networkRequest, disposeNetworkRequest] = makeNetworkRequest(environment, artifact, variables, null, null);
  if (readerArtifact == null) return;
  const fragmentReference = {
    kind: "FragmentReference",
    readerWithRefetchQueries: wrapResolvedValue({
      kind: "ReaderWithRefetchQueries",
      readerArtifact,
      nestedRefetchQueries,
    } as const),
    root: rootLink,
    variables,
    networkRequest,
  } as const;
  return [fragmentReference, disposeNetworkRequest];
};


const readerAst: ReaderAst<unknown> = [
];

const artifact: RefetchReaderArtifact = {
  kind: "RefetchReaderArtifact",
  // @ts-ignore
  resolver,
  readerAst,
};

export default artifact;
