#!/bin/sh
mkdir -p .test
mkdir -p .coverage
clarinet run --allow-write --allow-read --allow-env EXTRACT_CHECK ext/extract-error-codes.ts
