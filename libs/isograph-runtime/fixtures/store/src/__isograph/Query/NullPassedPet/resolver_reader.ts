import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__NullPassedPet__output_type } from "./output_type";
import type { Query__NullPassedPet__param } from "./param_type";
import { NullPassedPet as resolver } from "./../../../NullPassedPet";
import Pet__NullableCheckins__resolver_reader from "./../../Pet/NullableCheckins/resolver_reader";
const readerArtifact = (): ReaderArtifact<Query__NullPassedPet__param, Query__NullPassedPet__output_type, "Query"> =>({
    readerAst: [
      {
        kind: "Linked",
        alias: "pet",
        selectableName: "pet",
        arguments: [
          [
            "id",
            {
              kind: "Variable",
              name: "id"
            }
          ]
        ],
        selections: [
          {
            kind: "Resolver",
            alias: "NullableCheckins",
            arguments: [
              [
                "n",
                {
                  kind: "Literal",
                  value: null
                }
              ]
            ],
            readerArtifact: Pet__NullableCheckins__resolver_reader
          }
        ]
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
