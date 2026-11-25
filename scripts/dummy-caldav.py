"""Disposable loopback CalDAV accounts. Never reads the user's Taskroot data.

Install scripts/dummy-caldav-requirements.txt in a local venv, then run this
script with that venv's Python. --check performs a non-visual smoke test and exits.
"""

import argparse
import base64
import datetime
import pathlib
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
ACCOUNTS = {
    "taskroot-dummy-a": "dummy-a-only",
    "taskroot-dummy-b": "dummy-b-only",
}


def request(base, user, password, method, href, body=b"", headers=None):
    authorization = base64.b64encode(f"{user}:{password}".encode()).decode()
    request_headers = {"Authorization": f"Basic {authorization}", **(headers or {})}
    req = urllib.request.Request(base + href, body or None, request_headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=3) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def prepare(directory, port):
    directory.mkdir(parents=True, exist_ok=True)
    users = directory / "users"
    users.write_text("".join(f"{user}:{password}\n" for user, password in ACCOUNTS.items()), encoding="utf-8")
    config = directory / "config"
    config.write_text(
        f"[server]\nhosts = 127.0.0.1:{port}\n"
        f"[auth]\ntype = htpasswd\nhtpasswd_filename = {users.as_posix()}\nhtpasswd_encryption = plain\n"
        "[rights]\ntype = owner_only\n"
        f"[storage]\nfilesystem_folder = {(directory / 'storage').as_posix()}\n",
        encoding="utf-8",
    )
    return config


def await_server(process, base):
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError("Dummy Radicale exited; inspect its server.log")
        try:
            request(base, "taskroot-dummy-a", "dummy-a-only", "PROPFIND", "/", headers={"Depth": "0"})
            return
        except urllib.error.URLError:
            time.sleep(0.1)
    raise RuntimeError("Dummy Radicale did not become ready within 10 seconds")


def seed(base):
    fixtures = ROOT / "src-tauri/src/features/calendar/ics/fixtures"
    for user, password in ACCOUNTS.items():
        for collection, component, fixture in [
            ("tasks", "VTODO", "tasks-org-task.ics"),
            ("events", "VEVENT", "all-day-event.ics"),
        ]:
            href = f"/{user}/{collection}/"
            status, _ = request(base, user, password, "PROPFIND", href, headers={"Depth": "0"})
            if status != 207:
                xml = (
                    '<c:mkcalendar xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:d="DAV:">'
                    f'<d:set><d:prop><d:displayname>{user} {collection}</d:displayname>'
                    f'<c:supported-calendar-component-set><c:comp name="{component}"/>'
                    '</c:supported-calendar-component-set></d:prop></d:set></c:mkcalendar>'
                )
                status, body = request(base, user, password, "MKCALENDAR", href, xml.encode(), {"Content-Type": "application/xml"})
                if status not in (201, 405):
                    raise RuntimeError(f"MKCALENDAR {href}: {status} {body!r}")
            raw = (fixtures / fixture).read_text(encoding="utf-8")
            lines = []
            for line in raw.splitlines():
                if line.startswith("UID:"):
                    line = f"UID:dummy-{collection}"
                elif line.startswith("SUMMARY:"):
                    line = f"SUMMARY:{user} sample {collection}"
                elif line.startswith("DTSTART;VALUE=DATE:"):
                    line = f"DTSTART;VALUE=DATE:{datetime.date.today():%Y%m%d}"
                elif line.startswith("DTEND;VALUE=DATE:"):
                    line = f"DTEND;VALUE=DATE:{datetime.date.today() + datetime.timedelta(days=1):%Y%m%d}"
                lines.append(line)
            status, body = request(base, user, password, "PUT", href + "sample.ics",
                                   ("\r\n".join(lines) + "\r\n").encode(),
                                   {"Content-Type": "text/calendar"})
            if status not in (201, 204):
                raise RuntimeError(f"PUT {href}: {status} {body!r}")


def check(base):
    for user, password in ACCOUNTS.items():
        status, body = request(base, user, password, "GET", f"/{user}/tasks/sample.ics")
        if status != 200 or f"SUMMARY:{user} sample tasks".encode() not in body:
            raise RuntimeError(f"Fixture check failed for {user}: {status}")
    status, _ = request(base, "taskroot-dummy-a", "wrong", "GET", "/taskroot-dummy-a/tasks/sample.ics")
    if status != 401:
        raise RuntimeError("Dummy server must reject incorrect passwords")
    status, _ = request(base, "taskroot-dummy-a", "dummy-a-only", "GET", "/taskroot-dummy-b/tasks/sample.ics")
    if status not in (401, 403, 404):
        raise RuntimeError("Dummy users must not read each other's resources")
    print("PASS: both seeded accounts authenticate; incorrect passwords and cross-account reads are rejected.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=15232)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--check-backend", action="store_true", help="Run the opt-in Rust multi-account fixture contract, then stop")
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535 or args.port == 5232:
        parser.error("Use a disposable port (1024–65535), never the production port 5232")
    if args.check_backend and args.port != 15232:
        parser.error("The backend fixture contract uses port 15232")
    # Refuse to contact an existing listener, even if it looks like Radicale.
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", args.port))
    directory = ROOT / "src-tauri/target/dummy-caldav" / str(args.port)
    config = prepare(directory, args.port)
    base = f"http://127.0.0.1:{args.port}"
    with (directory / "server.log").open("a", encoding="utf-8") as log:
        process = subprocess.Popen([sys.executable, "-m", "radicale", "--config", str(config)], stdout=log, stderr=log)
        try:
            await_server(process, base)
            seed(base)
            check(base)
            if args.check_backend:
                subprocess.run(["cargo", "nextest", "run", "--run-ignored", "ignored-only", "-E",
                                "test(disposable_dummy_accounts_switch_real_resources_without_keychain_access)"],
                               cwd=ROOT / "src-tauri", check=True)
            if not (args.check or args.check_backend):
                print(f"Dummy CalDAV: {base}. Credentials are in AGENTS.md. Ctrl+C stops this fixture.")
                process.wait()
        except KeyboardInterrupt:
            pass
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


if __name__ == "__main__":
    main()
