/** @type {import('next').NextConfig} */
const nextConfig = {
  // To enable this flag, add code like
  // [this](https://github.com/facebook/relay/blob/c0cc17a07e1f0c01f3e5c564eed50b5a30f4228f/packages/react-relay/relay-hooks/useEntryPointLoader.js#L156-L189)
  // to useCachedPrecommitValue
  reactStrictMode: false,
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
};

module.exports = nextConfig;
