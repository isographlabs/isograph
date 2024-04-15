import type {ExtractSecondParam} from '@isograph/react';
import { meNameField as resolver } from '../../../garbageCollection.test.ts';
// the type, when read out (either via useLazyReference or via graph)
export type Query__meName__outputType = ReturnType<typeof resolver>;
