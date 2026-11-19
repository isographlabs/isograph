import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__Links__output_type } from "./output_type";
import type { Query__Links__param } from "./param_type";
import { Links as resolver } from "./../../../Links";
import Node__NodeLink__resolver_reader from "./../../Node/NodeLink/resolver_reader";
import Pet__PetLink__resolver_reader from "./../../Pet/PetLink/resolver_reader";
import PetStats__StatsLink__resolver_reader from "./../../PetStats/StatsLink/resolver_reader";
const readerArtifact = (): ReaderArtifact<Query__Links__param, Query__Links__output_type, "Query"> =>({
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
            alias: "PetLink",
            arguments: null,
            readerArtifact: Pet__PetLink__resolver_reader
          },
          {
            kind: "Linked",
            alias: "stats",
            selectableName: "stats",
            arguments: null,
            selections: [
              {
                kind: "Resolver",
                alias: "StatsLink",
                arguments: null,
                readerArtifact: PetStats__StatsLink__resolver_reader
              }
            ]
          },
          {
            kind: "Linked",
            alias: "bestFriend",
            selectableName: "bestFriend",
            arguments: null,
            selections: [
              {
                kind: "Resolver",
                alias: "PetLink",
                arguments: null,
                readerArtifact: Pet__PetLink__resolver_reader
              }
            ]
          }
        ]
      },
      {
        kind: "Linked",
        alias: "node",
        selectableName: "node",
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
            alias: "NodeLink",
            arguments: null,
            readerArtifact: Node__NodeLink__resolver_reader
          }
        ]
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
