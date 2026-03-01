import { mergeConfig, type UserConfig } from 'tsdown';
import baseConfig from '../../tsdown.config.mts';

const config: UserConfig = mergeConfig(baseConfig, {
  dts: {
    build: true,
  },
});
export default config;
