import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Checkin__make_super__param } from './param_type.ts';
import { Checkin__make_super__outputType } from './output_type.ts';
const includeReadOutData = (variables: any, readOutData: any) => {
  variables.checkin_id = readOutData.id;
  return variables;
};

import { makeNetworkRequest, type IsographEnvironment, type IsographEntrypoint } from '@isograph/react';
const resolver = (
  environment: IsographEnvironment,
  artifact: IsographEntrypoint<any, any>,
  readOutData: any,
  filteredVariables: any
) => (mutationParams: any) => {
  const variables = includeReadOutData({...filteredVariables, ...mutationParams}, readOutData);
  makeNetworkRequest(environment, artifact, variables);
};


const readerAst: ReaderAst<Checkin__make_super__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  Checkin__make_super__param,
  Checkin__make_super__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "make_super",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
