#!/usr/bin/env python3
"""Cross-platform Hermes session killer.

Finds and terminates Hermes agent processes by session ID.
Supports Linux (via /proc), macOS (via lsof), and Windows (via tasklist/taskkill).

Usage: python3 kill_session.py <session_id>
"""

import json
import os
import platform
import sqlite3
import signal  # needed for Unix kill
import subprocess
import sys
import time
from pathlib import Path


def get_hermes_home():
    return Path(os.environ.get("HERMES_HOME", Path.home() / ".hermes"))


# ── Platform detection ──────────────────────────────────────────────

IS_WINDOWS = platform.system() == "Windows"


def find_pids_with_open_file(file_path: str) -> list[int]:
    """Find PIDs that have the given file open, platform-aware."""
    if IS_WINDOWS:
        return _find_pids_windows(file_path)
    else:
        return _find_pids_unix(file_path)


def _find_pids_unix(file_path: str) -> list[int]:
    """Unix: use lsof or /proc for finding PIDs."""
    # Prefer lsof (works on macOS, Linux)
    try:
        result = subprocess.run(
            ["lsof", "-F", "p", file_path],
            capture_output=True, text=True, timeout=10
        )
        if result.returncode == 0:
            pids = set()
            for line in result.stdout.splitlines():
                if line.startswith("p"):
                    pid_str = line[1:].strip()
                    if pid_str.isdigit():
                        pids.add(int(pid_str))
            if pids:
                return sorted(pids)
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    # Fallback: /proc (Linux only)
    if platform.system() == "Linux":
        return _find_pids_proc(file_path)

    return []


def _find_pids_proc(file_path: str) -> list[int]:
    """Linux: scan /proc/<pid>/fd/ for open file handles."""
    pids = set()
    try:
        for proc_dir in Path("/proc").iterdir():
            if not proc_dir.name.isdigit():
                continue
            pid = int(proc_dir.name)
            try:
                fd_dir = proc_dir / "fd"
                if not fd_dir.is_dir():
                    continue
                for fd in fd_dir.iterdir():
                    try:
                        link = os.readlink(str(fd))
                        if link == file_path:
                            pids.add(pid)
                    except (OSError, FileNotFoundError):
                        pass
            except PermissionError:
                pass
    except FileNotFoundError:
        pass
    return sorted(pids)


def _find_pids_windows(file_path: str) -> list[int]:
    """Windows: use 'handle' utility or fallback to tasklist."""
    pids = []
    # Try using handle.exe from Sysinternals if available
    try:
        result = subprocess.run(
            ["handle", "-accepteula", "-nobanner", file_path],
            capture_output=True, text=True, timeout=10
        )
        for line in result.stdout.splitlines():
            # Format: "python.exe  pid: 12345   ..."
            parts = line.split()
            for i, part in enumerate(parts):
                if part.lower().startswith("pid:"):
                    pid_str = parts[i + 1] if i + 1 < len(parts) else ""
                    if pid_str.isdigit():
                        pids.append(int(pid_str))
        if pids:
            return sorted(set(pids))
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    # Fallback: find Python processes via tasklist
    try:
        result = subprocess.run(
            ["tasklist", "/FO", "CSV", "/NH", "/FI", "IMAGENAME eq python.exe"],
            capture_output=True, text=True, timeout=10
        )
        for line in result.stdout.splitlines():
            if not line.strip():
                continue
            parts = [p.strip('" ') for p in line.split(',')]
            if len(parts) >= 2 and parts[1].isdigit():
                pids.append(int(parts[1]))
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    return sorted(set(pids))


def is_hermes_process(pid: int) -> bool:
    """Check if a PID is a Hermes agent (not the dashboard or other tool)."""
    if IS_WINDOWS:
        return _is_hermes_windows(pid)
    else:
        return _is_hermes_unix(pid)


def _is_hermes_unix(pid: int) -> bool:
    """Unix: check /proc/<pid>/cmdline for hermes indicators."""
    try:
        cmdline = Path(f"/proc/{pid}/cmdline").read_text(errors="replace")
        comm = Path(f"/proc/{pid}/comm").read_text(errors="replace").strip()
        if "dashboard" in cmdline or "kill_session" in cmdline:
            return False
        if any(x in cmdline for x in ["hermes", "run_agent", "cli.py", "gateway"]):
            return True
        if any(x in comm for x in ["python"]):
            try:
                maps = Path(f"/proc/{pid}/maps").read_text(errors="replace")
                if "hermes" in maps.lower():
                    return True
            except Exception:
                pass
        return False
    except (FileNotFoundError, PermissionError, ProcessLookupError):
        return False


def _is_hermes_windows(pid: int) -> bool:
    """Windows: use wmic to check command line for hermes indicators."""
    try:
        result = subprocess.run(
            ["wmic", "process", "where", f"ProcessId={pid}", "get", "CommandLine", "/format:value"],
            capture_output=True, text=True, timeout=10
        )
        cmdline = result.stdout.lower()
        if "dashboard" in cmdline or "kill_session" in cmdline:
            return False
        if any(x in cmdline for x in ["hermes", "run_agent", "cli.py", "gateway"]):
            return True
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass
    return False


def send_stop_signal(session_id: str, state_db_path: str) -> dict:
    """Find and kill the Hermes process associated with a session."""
    result = {"killed": [], "errors": [], "session_id": session_id}

    pids = find_pids_with_open_file(state_db_path)

    if not pids:
        result["note"] = "No processes with state.db handles found"
        return result

    hermes_pids = [p for p in pids if is_hermes_process(p)]

    if not hermes_pids:
        result["note"] = "No Hermes agent processes found"
        return result

    for pid in hermes_pids:
        try:
            if IS_WINDOWS:
                subprocess.run(
                    ["taskkill", "/F", "/PID", str(pid)],
                    capture_output=True, timeout=10
                )
            else:
                os.kill(pid, signal.SIGTERM)
            result["killed"].append(pid)
        except (ProcessLookupError, OSError):
            result["errors"].append(f"PID {pid} already gone")
        except PermissionError:
            result["errors"].append(f"PID {pid} permission denied")

    # Also update state.db to mark session as ended (redundant safety)
    try:
        conn = sqlite3.connect(state_db_path)
        conn.execute(
            "UPDATE sessions SET ended_at = ?, end_reason = ? WHERE id = ? AND ended_at IS NULL",
            [time.time(), "dashboard_killed", session_id]
        )
        conn.commit()
        conn.close()
    except Exception as e:
        result["errors"].append(f"DB update: {e}")

    return result


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"status": "error", "message": "Usage: kill_session.py <session_id>"}))
        sys.exit(1)

    session_id = sys.argv[1]
    hermes_home = get_hermes_home()
    state_db = hermes_home / "state.db"

    if not state_db.exists():
        print(json.dumps({"status": "error", "message": f"state.db not found at {state_db}"}))
        sys.exit(1)

    result = send_stop_signal(session_id, str(state_db))
    result["status"] = "success" if result["killed"] else "info" if not result["errors"] else "partial"
    print(json.dumps(result))


if __name__ == "__main__":
    main()
