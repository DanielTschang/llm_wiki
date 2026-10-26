#!/usr/bin/env bash
# End to end: the real server over HTTP against the k3d Postgres and MinIO.
# Writes and reads a page, refuses host paths, streams a 12 MiB upload through
# staging into `raw/sources` and back out of `/files`, and extracts a PDF.
# Run `make test-pg` once first so the test database and bucket exist; build
# the server with `cargo build -p llm-wiki-server` in src-tauri.
set -uo pipefail
cd "$(dirname "$0")/../.."
S=$(mktemp -d)
K=(kubectl --context k3d-llm-wiki -n llm-wiki)
PGPORT=55433; S3PORT=59002; APIPORT=19877; TOKEN=e2e-token
cleanup() { kill ${PF1:-} ${PF2:-} ${SRV:-} 2>/dev/null; rm -rf "$S" "${DATA:-}"; }
trap cleanup EXIT

"${K[@]}" port-forward svc/pg-rw $PGPORT:5432 >/dev/null 2>&1 & PF1=$!
"${K[@]}" port-forward svc/minio $S3PORT:9000 >/dev/null 2>&1 & PF2=$!
sleep 3
PGPASS=$("${K[@]}" get secret pg-app -o jsonpath='{.data.password}' | base64 -d)
S3U=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_USER}' | base64 -d)
S3P=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_PASSWORD}' | base64 -d)

PROJECT=$("${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d llm_wiki_test -Atc "INSERT INTO projects (name) VALUES ('e2e') RETURNING id" | head -1)
echo "project=$PROJECT"

DATA=$(mktemp -d)
LLM_WIKI_DATA_DIR=$DATA LLM_WIKI_PORT=$APIPORT LLM_WIKI_TOKEN=$TOKEN LLM_WIKI_NO_WORKER=true \
LLM_WIKI_DATABASE_URL="postgres://llm_wiki:$PGPASS@127.0.0.1:$PGPORT/llm_wiki_test" \
LLM_WIKI_S3_ENDPOINT=http://127.0.0.1:$S3PORT LLM_WIKI_S3_BUCKET=llm-wiki-test LLM_WIKI_S3_ACCESS_KEY=$S3U LLM_WIKI_S3_SECRET_KEY=$S3P \
  src-tauri/target/debug/llm-wiki-server >$S/server.log 2>&1 & SRV=$!
for i in $(seq 1 50); do curl -s -o /dev/null http://127.0.0.1:$APIPORT/ && break; sleep 0.3; done

BASE=/projects/$PROJECT
rpc() { curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' "http://127.0.0.1:$APIPORT/rpc/$1" -d "$2"; echo; }
python3 - "$S/hello.pdf" <<'PY'
import sys
objs = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"]
text = b"BT /F1 24 Tf 20 100 Td (Hello platform) Tj ET"
objs += [b"<< /Length %d >>\nstream\n" % len(text) + text + b"\nendstream", b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"]
out, offs = b"%PDF-1.4\n", []
for i, o in enumerate(objs, 1):
    offs.append(len(out)); out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
x = len(out); out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
out += b"".join(b"%010d 00000 n \n" % o for o in offs)
out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, x)
open(sys.argv[1], "wb").write(out)
PY
echo "== write a page, read it back"
rpc write_file "{\"path\":\"$BASE/wiki/concepts/hello.md\",\"contents\":\"# Hello\\nplatform\"}"
rpc read_file "{\"path\":\"$BASE/wiki/concepts/hello.md\"}"
echo "== list wiki"
rpc list_directory "{\"path\":\"$BASE/wiki\"}"
echo "== keyword search and page links read the stored pages"
rpc write_file "{\"path\":\"$BASE/wiki/concepts/links.md\",\"contents\":\"# Links\\nsee [[hello]] about the platform\"}"
rpc search_project "{\"projectPath\":\"$BASE\",\"query\":\"platform\",\"topK\":5}" | python3 -c 'import json,sys; r=json.load(sys.stdin); print("search hits:", [h["path"] for h in r["results"]])'
rpc get_page_links "{\"projectPath\":\"$BASE\",\"filePath\":\"$BASE/wiki/concepts/links.md\"}"
echo "== .llm-wiki state files are documents in Postgres"
rpc write_file "{\"path\":\"$BASE/.llm-wiki/review.json\",\"contents\":\"[{\\\"id\\\":\\\"r1\\\"}]\"}"
rpc write_file "{\"path\":\"$BASE/.llm-wiki/chats/c1.json\",\"contents\":\"{\\\"messages\\\":[]}\"}"
rpc read_file "{\"path\":\"$BASE/.llm-wiki/review.json\"}"
rpc list_directory "{\"path\":\"$BASE/.llm-wiki\",\"includeHidden\":true}" | python3 -c 'import json,sys; print("state listing:", sorted(n["name"] for n in json.load(sys.stdin)))'
rpc file_exists "{\"path\":\"$BASE/.llm-wiki/ingest-queue.json\"}"
rpc write_file "{\"path\":\"$BASE/.llm-wiki/page-history/a-1.md\",\"contents\":\"x\"}"
rpc delete_file "{\"path\":\"$BASE/.llm-wiki/chats/c1.json\"}"
echo "== host paths are refused"
rpc read_file '{"path":"/etc/passwd"}'
rpc write_file "{\"path\":\"/tmp/pwned\",\"contents\":\"x\"}"
echo "== upload 12 MB to staging, copy into raw/sources, read size"
head -c 12582912 /dev/urandom > $S/big.bin
curl -s -X POST -H "Authorization: Bearer $TOKEN" --data-binary @$S/big.bin "http://127.0.0.1:$APIPORT/upload?batch=b1&name=big.bin&project=$PROJECT"; echo
rpc copy_file "{\"source\":\"$BASE/.uploads/b1/big.bin\",\"destination\":\"$BASE/raw/sources/big.bin\"}"
rpc get_file_size "{\"path\":\"$BASE/raw/sources/big.bin\"}"
echo "== download via /files and compare"
curl -s -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:$APIPORT/files?path=$BASE/raw/sources/big.bin" -o $S/big.out -D $S/big.hdr
shasum -a 256 $S/big.bin $S/big.out | awk '{print $1}' | uniq | wc -l | xargs echo "distinct hashes (1 = identical):"
grep -i -E "^content-(type|length)" $S/big.hdr
echo "== pdf: upload, preprocess, read"
curl -s -X POST -H "Authorization: Bearer $TOKEN" --data-binary @$S/hello.pdf "http://127.0.0.1:$APIPORT/upload?batch=b2&name=hello.pdf&project=$PROJECT"; echo
rpc copy_file "{\"source\":\"$BASE/.uploads/b2/hello.pdf\",\"destination\":\"$BASE/raw/sources/hello.pdf\"}"
rpc preprocess_file "{\"path\":\"$BASE/raw/sources/hello.pdf\"}"
rpc read_file "{\"path\":\"$BASE/raw/sources/hello.pdf\"}"
echo "== listing of raw/sources (cache is hidden)"
rpc list_directory "{\"path\":\"$BASE/raw/sources\"}"
echo "== unconverted command refuses instead of touching the disk"
rpc find_related_wiki_pages "{\"projectPath\":\"$BASE\",\"sourceName\":\"x\"}"
rpc list_file_history "{\"projectPath\":\"$BASE\"}"
rpc vector_count "{\"projectPath\":\"$BASE\"}"
echo "== what is in the database and the bucket"
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d llm_wiki_test -c "select 'pages' t, count(*) from pages where project_id='$PROJECT' and deleted_at is null union all select 'blobs', count(*) from blobs where project_id='$PROJECT' and deleted_at is null union all select 'state_files', count(*) from state_files where project_id='$PROJECT'"
echo "== server log"; tail -5 "$S/server.log"
