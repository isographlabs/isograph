
set -e

rm -rf ./out
npm run build

cd ./out

aws s3 rm s3://rb-pw-front/isograph-demo --recursive
aws s3 cp . s3://rb-pw-front/isograph-demo --acl public-read --recursive
aws s3 cp ./index.html s3://rb-pw-front/isograph-demo --acl public-read --cache-control max-age=0,no-cache --metadata-directive REPLACE
aws cloudfront create-invalidation --distribution-id E3QXR86BT07VQ8 --paths "/*"
