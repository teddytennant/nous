#!/bin/sh
# Real-libp2p, real-multi-process devnet demo:
# 1. Spawn 4 nous-pouw-node processes with HTTP RPC + initial balances.
# 2. Wait for the chain to advance + mesh to form.
# 3. Send a Transfer tx via the CLI hitting node 0's RPC.
# 4. Print balances on all 4 nodes; expect identical updates.

set -e
ROOT=/home/gradient/nous-pouw-work
NODE=$ROOT/target/release/examples/node
CLI=$ROOT/target/release/examples/pouw_cli

pkill -f 'examples/node' 2>/dev/null || true
sleep 1
rm -f /tmp/pouw-*.db /tmp/pouw-*.log

GB=10000  # genesis balance per validator

$NODE --listen /ip4/127.0.0.1/tcp/9601 --idx 0 --validators 4 \
  --genesis-balance $GB --rpc 127.0.0.1:8901 --db /tmp/pouw-0.db \
  > /tmp/pouw-0.log 2>&1 &
sleep 2
$NODE --listen /ip4/127.0.0.1/tcp/9602 --bootstrap /ip4/127.0.0.1/tcp/9601 \
  --idx 1 --validators 4 --genesis-balance $GB --rpc 127.0.0.1:8902 \
  --db /tmp/pouw-1.db > /tmp/pouw-1.log 2>&1 &
$NODE --listen /ip4/127.0.0.1/tcp/9603 --bootstrap /ip4/127.0.0.1/tcp/9601 \
  --idx 2 --validators 4 --genesis-balance $GB --rpc 127.0.0.1:8903 \
  --db /tmp/pouw-2.db > /tmp/pouw-2.log 2>&1 &
$NODE --listen /ip4/127.0.0.1/tcp/9604 --bootstrap /ip4/127.0.0.1/tcp/9601 \
  --idx 3 --validators 4 --genesis-balance $GB --rpc 127.0.0.1:8904 \
  --db /tmp/pouw-3.db > /tmp/pouw-3.log 2>&1 &

echo "waiting 20s for mesh + a few finalized blocks..."
sleep 20

echo
echo "=== status across all 4 nodes ==="
for p in 8901 8902 8903 8904; do
  echo "[node @ $p]"
  $CLI status --rpc http://127.0.0.1:$p
done

DID0=$($CLI did --idx 0)
DID1=$($CLI did --idx 1)
echo
echo "DID0=$DID0"
echo "DID1=$DID1"

echo
echo "=== balances BEFORE tx (recipient idx=1) ==="
for p in 8901 8902 8903 8904; do
  $CLI balance --idx 1 --rpc http://127.0.0.1:$p
done

echo
echo "=== submitting Transfer 250 from idx=0 -> idx=1 via node @ 8901 ==="
$CLI send-tx --from-idx 0 --to-idx 1 --amount 250 --nonce 1 \
  --rpc http://127.0.0.1:8901

echo "waiting 10s for tx to finalize across nodes..."
sleep 10

echo
echo "=== balances AFTER tx (recipient idx=1) — expect 10250 on all 4 ==="
for p in 8901 8902 8903 8904; do
  $CLI balance --idx 1 --rpc http://127.0.0.1:$p
done

echo
echo "=== sender (idx=0) — expect 9750 on all 4 ==="
for p in 8901 8902 8903 8904; do
  $CLI balance --idx 0 --rpc http://127.0.0.1:$p
done

pkill -f 'examples/node' 2>/dev/null || true
