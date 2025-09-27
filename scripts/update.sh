#!/bin/sh
set -e

git fetch upstream
git merge upstream/main

cd keos-projects/.cargo/template
git fetch origin
git merge origin/main
