import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__SpeciesRoute__output_type } from "./output_type";
import type { Query__SpeciesRoute__param } from "./param_type";
import { SpeciesRoute as resolver } from "./../../../SpeciesRoute";
import Pet__fullName__resolver_reader from "./../../Pet/fullName/resolver_reader";
const readerArtifact: ReaderArtifact<Query__SpeciesRoute__param, Query__SpeciesRoute__output_type, "Query"> = {
  readerAst: [
    {
      kind: "Linked",
      alias: "petsBySpecies",
      selectableName: "petsBySpecies",
      arguments: [
        [
          "species",
          {
            kind: "Variable",
            name: "species"
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
          alias: "fullName",
          arguments: null,
          readerArtifact: Pet__fullName__resolver_reader
        },
        {
          kind: "Scalar",
          alias: "favorite_phrase",
          selectableName: "favorite_phrase",
          arguments: null
        }
      ]
    },
    {
      kind: "Linked",
      alias: "viewer",
      selectableName: "viewer",
      arguments: null,
      selections: [
        {
          kind: "Scalar",
          alias: "id",
          selectableName: "id",
          arguments: null
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
            kind: "String",
            value: "1"
          }
        ]
      ],
      selections: [
        {
          kind: "Scalar",
          alias: "id",
          selectableName: "id",
          arguments: null
        }
      ]
    }
  ],
  resolver: resolver
};
export default readerArtifact;
