#!/bin/sh

url="https://raw.githubusercontent.com/github/rest-api-description/04fd6c592fc546217404b07e0b0e581fb00a963a/descriptions/api.github.com/api.github.com.json"

curl -sSL "$url" | jq -r '.paths | to_entries[] | .key as $path | .value | keys[] | "\(.):\($path)"' > benches/github_paths.txt