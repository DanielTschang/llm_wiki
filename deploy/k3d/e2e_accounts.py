#!/usr/bin/env python3
"""Drives a running platform server as several users and checks who may do what.

Environment: BASE (server URL), OPERATOR_TOKEN, ADMIN_USER, ADMIN_PASSWORD.
"""
import http.cookiejar
import json
import os
import sys
import urllib.error
import urllib.request
import uuid

import http.server
import threading

BASE = os.environ["BASE"]
OPERATOR_TOKEN = os.environ["OPERATOR_TOKEN"]
failures = []


def check(name, condition, detail=""):
    print(("  ok   " if condition else "  FAIL ") + name + ("" if condition else f"  {detail}"))
    if not condition:
        failures.append(name)


class Client:
    def __init__(self, bearer=None):
        self.bearer = bearer
        self.jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(self.jar), NoRedirect()
        )

    def _send(self, req):
        try:
            with self.opener.open(req) as response:
                return response.status, response.read()
        except urllib.error.HTTPError as error:
            return error.code, error.read()

    def _request(self, method, path, data=None, headers=None):
        headers = dict(headers or {})
        if self.bearer:
            headers["Authorization"] = f"Bearer {self.bearer}"
        else:
            headers.setdefault("X-LLM-Wiki-Client", "web")
        return urllib.request.Request(BASE + path, data=data, headers=headers, method=method)

    def login(self, username, password):
        body = urllib.parse.urlencode({"username": username, "password": password}).encode()
        status, _ = self._send(
            urllib.request.Request(BASE + "/login", data=body, method="POST")
        )
        return status

    def rpc(self, command, args=None):
        req = self._request(
            "POST", f"/rpc/{command}", json.dumps(args or {}).encode(),
            {"Content-Type": "application/json"},
        )
        status, body = self._send(req)
        try:
            return status, json.loads(body or b"null")
        except ValueError:
            return status, body.decode(errors="replace")

    def raw(self, method, path, data=None, headers=None):
        return self._send(self._request(method, path, data, headers))

    def get(self, path):
        return self._send(self._request("GET", path))

    def upload(self, project, batch, name, data):
        query = urllib.parse.urlencode({"batch": batch, "name": name, "project": project})
        return self._send(self._request("POST", f"/upload?{query}", data))


class Upstream(http.server.BaseHTTPRequestHandler):
    """A stand-in model server that records what it was sent."""
    seen = []

    def log_message(self, *args):
        pass

    def do_POST(self):
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length)
        Upstream.seen.append({"path": self.path, "headers": {k.lower(): v for k, v in self.headers.items()}, "body": body})
        if self.path.endswith("/embeddings"):
            payload = json.dumps({"data": [{"embedding": [0.25, 0.5, 0.75]}]}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for piece in ["Hel", "lo ", "there"]:
            self.wfile.write(f'data: {{"choices":[{{"delta":{{"content":"{piece}"}}}}]}}\n\n'.encode())
            self.wfile.flush()
        self.wfile.write(b'data: {"usage":{"prompt_tokens":11,"completion_tokens":5}}\n\ndata: [DONE]\n\n')


upstream = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
threading.Thread(target=upstream.serve_forever, daemon=True).start()
UPSTREAM = f"http://127.0.0.1:{upstream.server_port}"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


import urllib.parse  # noqa: E402  (used by Client above)

tag = uuid.uuid4().hex[:6]
admin = Client()
check("the first administrator signs in", admin.login(os.environ["ADMIN_USER"], os.environ["ADMIN_PASSWORD"]) == 303)
status, me = admin.rpc("whoami")
check("whoami says an admin user", status == 200 and me["user"]["isPlatformAdmin"], me)

print("accounts")
users = {}
for name, is_admin in [("alice", False), ("bob", False), ("carol", False), ("dana", True)]:
    status, made = admin.rpc("create_user", {
        "username": f"{name}-{tag}", "displayName": name.title(),
        "password": "a long enough password", "isPlatformAdmin": is_admin,
    })
    check(f"admin creates {name}", status == 200, made)
    users[name] = made
alice, bob, carol, dana = (Client() for _ in range(4))
for client, name in [(alice, "alice"), (bob, "bob"), (carol, "carol"), (dana, "dana")]:
    check(f"{name} signs in", client.login(f"{name}-{tag}", "a long enough password") == 303)
check("a weak password is refused", admin.rpc("create_user", {"username": f"weak-{tag}", "password": "short"})[0] == 400)
check("a duplicate username is refused", admin.rpc("create_user", {"username": f"ALICE-{tag}", "password": "a long enough password"})[0] == 409)
check("a user cannot create users", alice.rpc("create_user", {"username": f"x-{tag}", "password": "a long enough password"})[0] == 403)
check("a user cannot list all users", alice.rpc("list_users")[0] == 403)

print("projects")
check("a normal user cannot create a project", alice.rpc("create_project", {"name": "Nope"})[0] == 403)
status, project = admin.rpc("create_project", {"name": f"Lithography {tag}"})
check("a Platform Admin creates a project and owns it", status == 200 and project["role"] == "owner", project)
pid, base = project["id"], project["path"]
check("a new project starts with its schema, index, log and overview",
      all(admin.rpc("file_exists", {"path": f"{base}/{rel}"})[1] is True
          for rel in ["schema.md", "purpose.md", "wiki/index.md", "wiki/log.md", "wiki/overview.md"]))
check("a new project starts with its schema, index, log and overview",
      all(admin.rpc("file_exists", {"path": f"{base}/{rel}"})[1] is True for rel in ["schema.md", "purpose.md", "wiki/index.md", "wiki/log.md", "wiki/overview.md"]))
check("alice is not a member yet", alice.rpc("open_project", {"path": base})[0] == 403)
for name, role in [("alice", "editor"), ("bob", "viewer")]:
    check(f"admin adds {name} as {role}", admin.rpc("add_project_member", {"projectId": pid, "userId": users[name]["id"], "role": role})[0] == 200)
check("adding twice is a conflict", admin.rpc("add_project_member", {"projectId": pid, "userId": users["bob"]["id"], "role": "viewer"})[0] == 409)
check("alice (editor) cannot add members", alice.rpc("add_project_member", {"projectId": pid, "userId": users["carol"]["id"], "role": "viewer"})[0] == 403)
check("each user lists only their own projects",
      [p["id"] for p in alice.rpc("list_projects")[1]] == [pid] and carol.rpc("list_projects")[1] == [])
check("a non-member admin lists projects but has no role in them",
      all(p["role"] is None for p in dana.rpc("list_all_projects")[1] if p["id"] == pid))

print("content")
page = f"{base}/wiki/concepts/euv.md"
check("alice (editor) writes a page", alice.rpc("write_file", {"path": page, "contents": "# EUV\nextreme ultraviolet"})[0] == 200)
check("bob (viewer) reads it", bob.rpc("read_file", {"path": page})[1].startswith("# EUV"))
check("bob (viewer) cannot write", bob.rpc("write_file", {"path": page, "contents": "defaced"})[0] == 403)
check("bob (viewer) cannot create a directory", bob.rpc("create_directory", {"path": f"{base}/wiki/x"})[0] == 403)
check("carol (no role) cannot read", carol.rpc("read_file", {"path": page})[0] == 403)
check("carol cannot list", carol.rpc("list_directory", {"path": base})[0] == 403)
check("carol cannot search the project", carol.rpc("search_project", {"projectPath": base, "query": "ultraviolet"})[0] == 403)
check("bob (viewer) can search", len(bob.rpc("search_project", {"projectPath": base, "query": "ultraviolet"})[1]["results"]) == 1)
check("the same denial whether or not the project exists",
      carol.rpc("read_file", {"path": f"/projects/{uuid.uuid4()}/wiki/a.md"}) [0] == 403)
check("a page cannot be copied into a project the user cannot write",
      alice.rpc("copy_file", {"source": page, "destination": f"/projects/{uuid.uuid4()}/wiki/a.md"})[0] == 403)
check("an editor cannot delete a page", alice.rpc("delete_file", {"path": page})[0] == 403)
check("an editor can delete a source",
      alice.rpc("write_file", {"path": f"{base}/raw/sources/n.txt", "contents": "n"})[0] == 200
      and alice.rpc("delete_file", {"path": f"{base}/raw/sources/n.txt"})[0] == 200)
check("an owner can delete a page", admin.rpc("delete_file", {"path": page})[0] == 200)

print("files and uploads")
admin.rpc("write_file", {"path": f"{base}/wiki/a.md", "contents": "# A"})
check("bob (viewer) downloads a file", bob.get(f"/files?path={base}/wiki/a.md")[0] == 200)
check("carol is refused a download", carol.get(f"/files?path={base}/wiki/a.md")[0] == 403)
check("bob (viewer) cannot upload", bob.upload(pid, "b1", "x.bin", b"123")[0] == 403)
check("alice (editor) can upload", alice.upload(pid, "b1", "x.bin", b"123")[0] == 200)
check("carol cannot upload", carol.upload(pid, "b2", "x.bin", b"123")[0] == 403)

print("members")
status, members = bob.rpc("list_project_members", {"projectId": pid})
check("a viewer lists the members", status == 200 and {m["role"] for m in members} == {"owner", "editor", "viewer"}, members)
check("an outsider cannot", carol.rpc("list_project_members", {"projectId": pid})[0] == 403)
check("the only owner cannot be removed", admin.rpc("remove_project_member", {"projectId": pid, "userId": me["user"]["id"]})[0] == 403)
check("bob can leave a project", bob.rpc("remove_project_member", {"projectId": pid, "userId": users["bob"]["id"]})[0] == 200)
check("and then has no access", bob.rpc("read_file", {"path": f"{base}/wiki/a.md"})[0] == 403)
check("an admin who is not a member can add themselves",
      dana.rpc("add_project_member", {"projectId": pid, "userId": users["dana"]["id"], "role": "viewer"})[0] == 200
      and dana.rpc("read_file", {"path": f"{base}/wiki/a.md"})[0] == 200)
check("lookup finds a user by username", carol.rpc("lookup_user", {"username": f"ALICE-{tag}"})[1]["displayName"] == "Alice")

print("accounts, continued")
check("a wrong current password is refused", alice.rpc("change_my_password", {"currentPassword": "nope nope nope", "newPassword": "another long password"})[0] == 403)
check("alice changes her password", alice.rpc("change_my_password", {"currentPassword": "a long enough password", "newPassword": "another long password"})[0] == 200)
check("her old session is gone", alice.rpc("whoami")[0] == 401)
check("the new password works", Client().login(f"alice-{tag}", "another long password") == 303)
check("admin disables bob", admin.rpc("set_user_disabled", {"userId": users["bob"]["id"], "disabled": True})[0] == 200)
check("his session ends", bob.rpc("whoami")[0] == 401)
check("and he cannot sign in", Client().login(f"bob-{tag}", "a long enough password") == 403)

alice2 = Client()
check("alice signs in again with the new password", alice2.login(f"alice-{tag}", "another long password") == 303)

print("settings")
check("a user keeps their own last project", alice2.rpc("app_store_set", {"store": "app-state.json", "key": "lastProject", "value": {"path": base}})[0] == 200)
check("another user does not see it", carol.rpc("app_store_get", {"store": "app-state.json", "key": "lastProject"})[1] is None)
check("and gets it back themselves", alice2.rpc("app_store_get", {"store": "app-state.json", "key": "lastProject"})[1]["path"] == base)
check("a user cannot change organisation settings", alice2.rpc("app_store_set", {"store": "app-state.json", "key": "taskModelRouting", "value": {"chat": "x"}})[0] == 403)
check("an administrator can", admin.rpc("app_store_set", {"store": "app-state.json", "key": "taskModelRouting", "value": {"chat": "m1"}})[0] == 200)
check("and everyone reads them", carol.rpc("app_store_get", {"store": "app-state.json", "key": "taskModelRouting"})[1]["chat"] == "m1")
check("a user cannot change the proxy", alice2.rpc("set_proxy_env", {})[0] == 403)

status, second = admin.rpc("create_project", {"name": f"Gateway {tag}"})
pid2 = second["id"]
admin.rpc("add_project_member", {"projectId": pid2, "userId": users["alice"]["id"], "role": "viewer"})
print("llm gateway")
models = [
    {"id": "stub-chat", "label": "Stub chat", "provider": "custom", "model": "stub-1",
     "baseUrl": UPSTREAM + "/v1", "apiKeyEnv": "LLM_WIKI_KEY_STUB", "default": True},
    {"id": "stub-embed", "label": "Stub embeddings", "purpose": "embedding", "provider": "custom",
     "model": "embed-1", "baseUrl": UPSTREAM + "/v1/embeddings", "apiKeyEnv": "LLM_WIKI_KEY_STUB",
     "outputDimensionality": 3, "default": True},
    {"id": "no-key", "label": "Key missing", "provider": "custom", "model": "x",
     "baseUrl": UPSTREAM + "/other", "apiKeyEnv": "LLM_WIKI_KEY_NOT_SET"},
]
check("a user cannot change the models", alice2.rpc("set_llm_models", {"models": models})[0] == 403)
check("a bad key variable is refused", admin.rpc("set_llm_models", {"models": [{**models[0], "apiKeyEnv": "DATABASE_URL"}]})[0] == 400)
check("a built-in provider cannot be pointed elsewhere", admin.rpc("set_llm_models", {"models": [{"id": "x", "label": "x", "provider": "openai", "model": "m", "baseUrl": "https://evil.example"}]})[0] == 400)
check("the administrator sets the models", admin.rpc("set_llm_models", {"models": models})[0] == 200)
status, listed = alice2.rpc("list_llm_models")
check("a member lists them without the key variable", status == 200 and len(listed) == 3 and all("apiKeyEnv" not in m for m in listed), listed)
check("the administrator also sees how each key is found", all("apiKeyEnv" in m for m in admin.rpc("list_llm_models")[1]))
check("the administrator sees which keys the server has",
      {m["id"]: m["keyConfigured"] for m in admin.rpc("list_llm_models")[1]} == {"stub-chat": True, "stub-embed": True, "no-key": False})

relay_headers = {
    "Content-Type": "application/json",
    "Authorization": "Bearer placeholder",
    "X-LLM-Target": UPSTREAM + "/v1/chat/completions",
    "X-LLM-Project": pid2,
}
Upstream.seen.clear()
status, body = alice2.raw("POST", "/llm/relay/stub-chat", json.dumps({"messages": [{"role": "user", "content": "hi"}]}).encode(), relay_headers)
check("a member's request is relayed and streams back", status == 200 and b"Hel" in body and b"there" in body and b"[DONE]" in body, (status, body[:80]))
seen = Upstream.seen[-1] if Upstream.seen else {"headers": {}, "body": b"", "path": ""}
check("the server's key went upstream, not the browser's", seen["headers"].get("authorization") == "Bearer sk-stub-secret", seen["headers"])
check("nothing of the session went upstream", "cookie" not in seen["headers"] and "x-llm-project" not in seen["headers"] and "x-llm-target" not in seen["headers"])
check("the request body arrived unchanged", json.loads(seen["body"])["messages"][0]["content"] == "hi")

def relay(client, model, target, **extra):
    h = {"Content-Type": "application/json", "Authorization": "Bearer placeholder", "X-LLM-Target": target, **extra}
    return client.raw("POST", f"/llm/relay/{model}", b"{}", h)[0]

check("a target outside the model's address is refused", relay(alice2, "stub-chat", "http://127.0.0.1:1/v1/chat/completions") == 403)
check("so is another path on the same server", relay(alice2, "stub-chat", UPSTREAM + "/admin") == 403)
check("so is a lookalike host", relay(alice2, "stub-chat", "http://127.0.0.1.evil.example/v1/chat") == 403)
check("an unknown model is not found", relay(alice2, "nope", UPSTREAM + "/v1/chat") == 404)
check("a model whose key is missing says so", relay(alice2, "no-key", UPSTREAM + "/other/chat") == 503)
check("only POST is relayed", alice2.raw("GET", "/llm/relay/stub-chat")[0] == 405)
check("a project the caller is not in is refused", relay(carol, "stub-chat", UPSTREAM + "/v1/chat/completions", **{"X-LLM-Project": pid2}) == 403)
check("a stranger is not signed in", Client().raw("POST", "/llm/relay/stub-chat", b"{}")[0] == 401)
check("members cannot use the open proxy", alice2.raw("POST", "/proxy?url=" + urllib.parse.quote(UPSTREAM + "/v1/chat"), b"{}", {"Content-Type": "application/json"})[0] == 403)

import time
time.sleep(0.5)
status, usage = admin.rpc("llm_usage_summary", {"days": 1})
mine = [u for u in usage if u["modelId"] == "stub-chat" and u["displayName"] and u["displayName"].lower().startswith("alice")]
check("the call is counted with the tokens the model reported",
      status == 200 and mine and mine[0]["calls"] >= 1 and mine[0]["inputTokens"] == 11 and mine[0]["outputTokens"] == 5, usage)
check("only an administrator sees usage", alice2.rpc("llm_usage_summary")[0] == 403)

print("keys stay on the server")
check("model settings read as empty", alice2.rpc("app_store_get", {"store": "app-state.json", "key": "llmConfig"})[1] is None)
check("and cannot be stored in the settings", admin.rpc("app_store_set", {"store": "app-state.json", "key": "llmConfig", "value": {"apiKey": "sk-leak"}})[0] == 403)
Upstream.seen.clear()
check("an embedding request must name a server model", alice2.rpc("embedding_fetch", {"text": "hi", "cfg": {"enabled": True, "endpoint": UPSTREAM, "apiKey": "sk-mine", "model": "m"}})[0] == 400)
status, vector = alice2.rpc("embedding_fetch", {"text": "hi", "cfg": {"enabled": True, "endpoint": "http://evil.example", "apiKey": "sk-mine", "model": "m", "serverModelId": "stub-embed"}})
check("a named embedding model is called with the server's key and address",
      status == 200 and vector == [0.25, 0.5, 0.75] and Upstream.seen and Upstream.seen[-1]["headers"].get("authorization") == "Bearer sk-stub-secret", (status, vector))

print("operator token")
operator = Client(bearer=OPERATOR_TOKEN)
check("the operator needs an owner to create a project", operator.rpc("create_project", {"name": "Ops"})[0] == 400)
status, ops = operator.rpc("create_project", {"name": f"Ops {tag}", "ownerUsername": f"dana-{tag}"})
check("the operator creates a project for a named admin", status == 200, ops)
check("the operator reads any project", operator.rpc("read_file", {"path": f"{base}/wiki/a.md"})[1] == "# A")

print("deleting a project")
check("an editor cannot delete it", alice.rpc("delete_project", {"projectId": pid})[0] in (401, 403))
check("an editor cannot delete it (signed in again)", alice2.rpc("delete_project", {"projectId": pid})[0] == 403)
check("the owner deletes it", admin.rpc("delete_project", {"projectId": pid})[0] == 200)
check("it is gone for everyone", alice2.rpc("read_file", {"path": f"{base}/wiki/a.md"})[0] == 403 and operator.rpc("open_project", {"path": base})[0] == 404)

print()
if failures:
    print(f"{len(failures)} check(s) failed")
    sys.exit(1)
print("all checks passed")
