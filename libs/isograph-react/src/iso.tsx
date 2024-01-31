type IdentityWithParam<TParam> = <TResolverReturn>(
  x: (param: TParam) => TResolverReturn,
) => (param: TParam) => TResolverReturn;

type RepositoryLinkParam = {
  data: {
    id: string;
    name: string;
    owner: {
      login: string;
    };
  };
  [index: string]: any;
};

export type RepositoryPageType = {
  data: {
    // MEH
    Header: any;
    // MEH times two
    RepositoryDetail: any;
  };
  [index: string]: any;
};

type UserLinkParams = {
  data: {
    login: string;
  };
  [index: string]: any;
};

type WhitespaceCharacter = ' ' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}` ? Whitespace<In> : In;

type MatchesWhitespaceAndString<TString extends string, T> =
  Whitespace<T> extends `${TString}${string}` ? T : never;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Repository.RepositoryLink', T>,
): IdentityWithParam<RepositoryLinkParam>;
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.RepositoryPage', T>,
): IdentityWithParam<RepositoryPageType>;
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Actor.UserLink', T>,
): IdentityWithParam<UserLinkParams>;

export function iso(_queryText: string): IdentityWithParam<any> {
  // The name `identity` here is a bit of a double entendre.
  // First, it is the identity function, constrained to operate
  // on a very specific type. Thus, the value of b Declare`...`(
  // someFunction) is someFunction. But furthermore, if one
  // write b Declare`...` and passes no function, the resolver itself
  // is the identity function. At that point, the types
  // TResolverParameter and TResolverReturn must be identical.

  return function identity<TResolverReturn>(
    x: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return x;
  };
}
