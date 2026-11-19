import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__NestedQuery__output_type } from "./output_type";
import type { Query__NestedQuery__param } from "./param_type";
import { NestedQuery as resolver } from "./../../../NestedQuery";
const readerArtifact: ReaderArtifact<Query__NestedQuery__param, Query__NestedQuery__output_type, "Query"> = {
  readerAst: [
    {
      kind: "Scalar",
      alias: "greeting",
      selectableName: "greeting",
      arguments: null
    },
    {
      kind: "Linked",
      alias: "query",
      selectableName: "query",
      arguments: null,
      selections: [
        {
          kind: "Scalar",
          alias: "tags",
          selectableName: "tags",
          arguments: null
        }
      ]
    }
  ],
  resolver: resolver
};
export default readerArtifact;
