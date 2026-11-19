import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__fullName__output_type } from "./output_type";
import type { Pet__fullName__param } from "./param_type";
import { fullName as resolver } from "./../../../Pet";
const readerArtifact = (): ReaderArtifact<Pet__fullName__param, Pet__fullName__output_type, "Pet"> =>({
    readerAst: [
      {
        kind: "Scalar",
        alias: "firstName",
        selectableName: "firstName",
        arguments: null
      },
      {
        kind: "Scalar",
        alias: "lastName",
        selectableName: "lastName",
        arguments: null
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
