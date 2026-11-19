import type { ReaderArtifact } from "@isograph/runtime";
import type { PetStats__StatsLink__output_type } from "./output_type";
import type { PetStats__StatsLink__param } from "./param_type";
import { StatsLink as resolver } from "./../../../StatsLink";
const readerArtifact = (): ReaderArtifact<PetStats__StatsLink__param, PetStats__StatsLink__output_type, "PetStats"> =>({
    readerAst: [],
    resolver: resolver
  });
export default readerArtifact;
