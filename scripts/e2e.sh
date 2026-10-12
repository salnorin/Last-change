#!/usr/bin/env bash
# End-to-end: two `lastchange serve` nodes; post via node A's HTTP API, read it on node B,
# then run the full Proof of Trust + rating flow across nodes.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q -p lastchange-node
B=target/debug/lastchange; T=$(mktemp -d); trap 'kill $(jobs -p) 2>/dev/null; cat $T/a.log $T/b.log; rm -rf $T' EXIT
export LASTCHANGE_PASSPHRASE=e2e-test-passphrase
$B --dir $T/a keygen >/dev/null; $B --dir $T/b keygen >/dev/null
$B --dir $T/a --stamp-bits 8 serve --http 127.0.0.1:28081 --listen /ip4/127.0.0.1/tcp/24701 > $T/a.log 2>&1 &
sleep 2; PA=$(grep -m1 -o 'peer .*' $T/a.log || true)
ADDR=/ip4/127.0.0.1/tcp/24701
$B --dir $T/b --stamp-bits 8 serve --http 127.0.0.1:28082 --listen /ip4/127.0.0.1/tcp/0 --dial $ADDR > $T/b.log 2>&1 &
sleep 4
export NO_PROXY=127.0.0.1 no_proxy=127.0.0.1
j() { python3 -c "import sys,json;print(json.load(sys.stdin)$1)"; }
A=http://127.0.0.1:28081/api; Bu=http://127.0.0.1:28082/api
BPK=$(curl -s $Bu/whoami | j "['pubkey']")
L=$(curl -s -XPOST $A/post -H 'content-type: application/json' -d '{"kind":"offer","title":"Oil painting, sunset","category":"art","images":["bafy-demo-hash"]}' | j "['id']")
for i in $(seq 20); do curl -s "$Bu/listings?category=art" | grep -q "$L" && break; sleep 1; done
curl -s "$Bu/listings?category=art" | grep -q "$L" && echo "OK listing propagated A->B"
ACC=$(curl -s -XPOST $A/accept -H 'content-type: application/json' -d "{\"listing_id\":\"$L\",\"buyer\":\"$BPK\"}" | j "['id']")
sleep 3
curl -s -XPOST $Bu/step -H 'content-type: application/json' -d "{\"accept_id\":\"$ACC\",\"step\":\"buyer_paid\"}" >/dev/null
R=$(curl -s -XPOST $A/complete -H 'content-type: application/json' -d "{\"listing_id\":\"$L\",\"buyer\":\"$BPK\"}" | j "['id']")
for i in $(seq 20); do curl -s -XPOST $Bu/rate -H 'content-type: application/json' -d "{\"receipt_id\":\"$R\",\"score\":5}" | grep -q '"id"' && break; sleep 1; done
APK=$(curl -s $A/whoami | j "['pubkey']")
for i in $(seq 20); do curl -s $A/trust/$APK | grep -q '"count":1' && break; sleep 1; done
curl -s $A/trust/$APK | grep -q '"count":1' && echo "OK rating from B counted on A: $(curl -s $A/trust/$APK)"
curl -s http://127.0.0.1:28081/ | grep -q Last-change && echo "OK web app served"
