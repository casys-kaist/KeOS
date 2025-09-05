#!/bin/sh
set -e

git fetch upstream
git merge upstream/main
