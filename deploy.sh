#!/usr/bin/env sh

view_url=http://localhost:8081

dir=$1
commit_id=$(git rev-parse HEAD)

echo "Deploying files for commit ${commit_id} using files from dir \"${dir}\"..."

data=''

for file in $(find $1 -type f); do
  hash=$(sha256sum $file | cut -d ' ' -f 1)
  data+="{\"path\":\"$(realpath --relative-to=$dir $file)\",\"object_id\":\"${hash}\"},"
done

echo $data
result=$(curl -d "[${data::-1}]" -H 'Content-Type: application/json' -X PUT "${view_url}/commit/${commit_id}")
echo $result

for file in $(echo $result | jq -c '.[]'); do
  path=$(echo $file | jq -r '.path')
  object_id=$(echo $file | jq -r '.object_id')
  echo $path $object_id
  curl -X PUT -F "file=@$dir/${path}" "${view_url}/object/${object_id}"
done
