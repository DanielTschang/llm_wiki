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

    def get(self, path):
        return self._send(self._request("GET", path))

    def upload(self, project, batch, name, data):
        query = urllib.parse.urlencode({"batch": batch, "name": name, "project": project})
        return self._send(self._request("POST", f"/upload?{query}", data))


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
check("a wrong current password is refused", alice.rpc("change_my_password", {"currentPassword": "nope nope nope", "newPassword": "another long password"})[0] == 401)
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
check("a user cannot change organisation settings", alice2.rpc("app_store_set", {"store": "app-state.json", "key": "llmConfig", "value": {"model": "x"}})[0] == 403)
check("an administrator can", admin.rpc("app_store_set", {"store": "app-state.json", "key": "llmConfig", "value": {"model": "m1"}})[0] == 200)
check("and everyone reads them", carol.rpc("app_store_get", {"store": "app-state.json", "key": "llmConfig"})[1]["model"] == "m1")
check("a user cannot change the proxy", alice2.rpc("set_proxy_env", {})[0] == 403)

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
