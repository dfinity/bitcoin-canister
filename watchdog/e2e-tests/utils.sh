#!/usr/bin/env bash

function cleanup {
  dfx stop
}

function error {
  echo "An error occurred in the script"
  cleanup
  exit 1
}
