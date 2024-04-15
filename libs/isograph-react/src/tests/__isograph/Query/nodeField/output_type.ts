import type {ExtractSecondParam} from '@isograph/react';
import { nodeField as resolver } from '../../../nodeQuery.ts';
// the type, when read out (either via useLazyReference or via graph)
export type Query__nodeField__outputType = ReturnType<typeof resolver>;
