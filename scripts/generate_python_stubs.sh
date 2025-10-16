#!/usr/bin/env bash
set -euo pipefail

# Generate Python gRPC stubs for Execution proto
# Requires: python -m pip install grpcio grpcio-tools

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
PROTO_DIR="$ROOT_DIR/proto"
OUT_DIR="$ROOT_DIR/sdks/python/poseidon_wep_sdk_pkg/generated"

python -m grpc_tools.protoc \
  -I"$PROTO_DIR" \
  --python_out="$OUT_DIR" \
  --grpc_python_out="$OUT_DIR" \
  "$PROTO_DIR/execution/v1/execution.proto"

echo "Generated Python stubs into: $OUT_DIR"
