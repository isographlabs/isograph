import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__PetSummary__output_type } from "./output_type";
import type { Query__PetSummary__param } from "./param_type";
import { PetSummary as resolver } from "./../../../PetSummary";
import Pet__PetName__resolver_reader from "./../../Pet/PetName/resolver_reader";
import Pet__RecentCheckins__resolver_reader from "./../../Pet/RecentCheckins/resolver_reader";
const readerArtifact = (): ReaderArtifact<Query__PetSummary__param, Query__PetSummary__output_type, "Query"> =>({
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
            kind: "Scalar",
            alias: "id",
            selectableName: "id",
            arguments: null
          },
          {
            kind: "Resolver",
            alias: "PetName",
            arguments: null,
            readerArtifact: Pet__PetName__resolver_reader
          },
          {
            kind: "Resolver",
            alias: "RecentCheckins",
            arguments: [
              [
                "n",
                {
                  kind: "Literal",
                  value: 2
                }
              ]
            ],
            readerArtifact: Pet__RecentCheckins__resolver_reader
          }
        ]
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
