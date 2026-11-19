export type ResolverParameters<TData> = { data: TData };

export function iso(_literal: string) {
  return <TResolver>(resolver: TResolver): TResolver => resolver;
}
