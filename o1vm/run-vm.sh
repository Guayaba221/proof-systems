#!/usr/bin/env bash
set -euo pipefail

cargo run --bin o1vm \
    --all-features \
    --release \
    -p o1vm -- \
      --pprof.cpu \
      --info-at "${INFO_AT:-%10000000}" \
      --snapshot-state-at "${SNAPSHOT_STATE_AT:-%10000000}" \
      --proof-at never \
      --stop-at "${STOP_AT:-never}" \
      --input "${ZKVM_STATE_FILENAME:-./state.json}" \
      -- \
      ./ethereum-optimism/op-program/bin/op-program \
      --log.level DEBUG \
      --l1 "${L1_RPC}" \
      --l2 "${L2_RPC}" \
      --network sepolia \
      --datadir "${OP_PROGRAM_DATA_DIR}" \
      --l1.head "${L1_HEAD}" \
      --l2.head "${L2_HEAD}" \
      --l2.outputroot "${STARTING_OUTPUT_ROOT}" \
      --l2.claim "${L2_CLAIM}" \
      --l2.blocknumber "${L2_BLOCK_NUMBER}" \
      --server
