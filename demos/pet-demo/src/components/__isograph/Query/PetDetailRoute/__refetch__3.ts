import type {IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
const queryText = 'mutation Checkinmake_super ($checkin_id: ID!) {\
make_super____checkin_id___v_checkin_id: make_checkin_super(checkin_id: $checkin_id) {\
checkin { \
  id,\
  location,\
  time,\
}}}';

const normalizationAst: NormalizationAst = [{
  kind: "Linked",
  fieldName: "make_super",
  arguments: [
    [
      "checkin_id",
      { kind: "Variable", name: "checkin_id" },
    ],
  ],
  selections: [
    {
      kind: "Linked",
      fieldName: "checkin",
      arguments: null,
      selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "location",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "time",
        arguments: null,
      },
    ],
    },
  ],
}];
const artifact: any = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
