#!/usr/bin/env python3
"""Real Hermes session compression - compresses session history using Ollama.

Usage: python3 compress_session.py <session_id>

This script:
1. Reads session messages from state.db FTS tables
2. Calls the configured compression model (deepseek-r1:7b via Ollama)
3. Creates a compressed summary
4. Stores the result in state.db
"""

import json
import os
import sqlite3
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path


def get_hermes_home():
    return Path(os.environ.get("HERMES_HOME", Path.home() / ".hermes"))


def get_ollama_config():
    """Get compression model config from hermes config.yaml."""
    config_path = get_hermes_home() / "config.yaml"
    if not config_path.exists():
        return {
            "base_url": "http://127.0.0.1:11434/v1",
            "model": "deephermes3:8b-preview",
            "api_key": "ollama",
        }
    try:
        import yaml
        with open(config_path) as f:
            config = yaml.safe_load(f)
        aux = config.get("auxiliary", {}).get("compression", {})
        return {
            "base_url": aux.get("base_url", "http://127.0.0.1:11434/v1"),
            "model": aux.get("model", "deepseek-r1:7b"),
            "api_key": aux.get("api_key", "ollama"),
        }
    except Exception:
        return {
            "base_url": "http://127.0.0.1:11434/v1",
            "model": "deephermes3:8b-preview",
            "api_key": "ollama",
        }


def read_session_messages(state_db: str, session_id: str) -> list[dict]:
    """Read session messages from state.db using FTS search."""
    conn = sqlite3.connect(state_db)
    conn.row_factory = sqlite3.Row
    messages = []

    try:
        # Get session info first
        cur = conn.execute(
            "SELECT id, model, started_at, ended_at, message_count, api_call_count, "
            "input_tokens, output_tokens, title FROM sessions WHERE id = ?",
            (session_id,)
        )
        session_info = dict(cur.fetchone() or {})

        # Try to get messages via FTS
        cur = conn.execute(
            "SELECT content, role, message_idx, timestamp FROM messages "
            "WHERE session_id = ? ORDER BY message_idx ASC LIMIT 500",
            (session_id,)
        )
        for row in cur.fetchall():
            messages.append(dict(row))

    except sqlite3.OperationalError as e:
        # messages table might not exist
        session_info = {"error": str(e)}
    finally:
        conn.close()

    return messages, session_info


def call_ollama_compress(messages: list[dict], ollama_config: dict, session_info: dict) -> dict:
    """Call Ollama API to compress session history."""
    if not messages:
        return {"error": "No messages to compress"}

    # Build a compact representation of the conversation
    conversation_lines = []
    for msg in messages[:100]:  # Limit to first 100 messages
        role = msg.get("role", "unknown")
        content = msg.get("content", "")
        # Truncate long messages
        if len(content) > 500:
            content = content[:500] + "..."
        conversation_lines.append(f"[{role}] {content}")

    conversation_text = "\n".join(conversation_lines)
    if len(conversation_text) > 15000:
        conversation_text = conversation_text[:15000] + "\n[... truncated ...]"

    prompt = f"""Compress the following AI assistant conversation session into a concise summary. 
Include: what was discussed, key decisions made, technical approaches considered, and outcomes.
Keep it under 500 tokens worth of text. This summary will be used by the AI to recall the session.

Session: {session_info.get('id', 'unknown')}
Model: {session_info.get('model', 'unknown')}
Total messages: {len(messages)}

Conversation:
{conversation_text}

Compressed summary:"""

    # Call Ollama API (OpenAI-compatible endpoint)
    url = f"{ollama_config['base_url'].rstrip('/')}/chat/completions"
    payload = json.dumps({
        "model": ollama_config["model"],
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 1024,
        "temperature": 0.3,
    }).encode("utf-8")

    req = urllib.request.Request(
        url,
        data=payload,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {ollama_config.get('api_key', 'ollama')}",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=180) as resp:
            result = json.loads(resp.read())
            summary = result.get("choices", [{}])[0].get("message", {}).get("content", "")
            reasoning = result.get("choices", [{}])[0].get("message", {}).get("reasoning", "")
            return {
                "summary": summary,
                "reasoning": reasoning or None,
                "model": ollama_config["model"],
                "tokens": {
                    "input": len(conversation_text.split()),
                    "output": len(summary.split()),
                },
            }
    except urllib.error.HTTPError as e:
        return {"error": f"HTTP {e.code}: {e.read().decode(errors='replace')[:200]}"}
    except Exception as e:
        return {"error": str(e)}


def store_compression(state_db: str, session_id: str, summary: dict):
    """Store the compression result back in state.db."""
    if "error" in summary:
        return {"stored": False, "error": summary["error"]}

    conn = sqlite3.connect(state_db)
    try:
        # Update session end_reason to 'compression' with actual compression
        now = time.time()
        conn.execute(
            "UPDATE sessions SET end_reason = 'compression', ended_at = ?1 "
            "WHERE id = ?2 AND ended_at IS NULL",
            (now, session_id)
        )

        # Store compressed summary in a custom note (using title to store it)
        compressed_text = summary.get("summary", "")
        if compressed_text:
            # Try to store in a compression_data table (if exists) or add metadata
            try:
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS compression_data ("
                    "session_id TEXT PRIMARY KEY, "
                    "summary TEXT, "
                    "reasoning TEXT, "
                    "model TEXT, "
                    "input_words INTEGER DEFAULT 0, "
                    "output_words INTEGER DEFAULT 0, "
                    "compressed_at REAL"
                    ")"
                )
                conn.execute(
                    "INSERT OR REPLACE INTO compression_data "
                    "(session_id, summary, reasoning, model, input_words, output_words, compressed_at) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    (
                        session_id,
                        compressed_text,
                        summary.get("reasoning", ""),
                        summary.get("model", "unknown"),
                        summary.get("tokens", {}).get("input", 0),
                        summary.get("tokens", {}).get("output", 0),
                        now,
                    )
                )
            except Exception:
                pass  # Non-critical

        conn.commit()
        return {"stored": True, "session_id": session_id}
    except Exception as e:
        conn.rollback()
        return {"stored": False, "error": str(e)}
    finally:
        conn.close()


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"status": "error", "message": "Usage: compress_session.py <session_id>"}))
        sys.exit(1)

    session_id = sys.argv[1]
    hermes_home = get_hermes_home()
    state_db = hermes_home / "state.db"

    if not state_db.exists():
        print(json.dumps({"status": "error", "message": f"state.db not found at {state_db}"}))
        sys.exit(1)

    ollama_config = get_ollama_config()

    # Step 1: Read session messages
    messages, session_info = read_session_messages(str(state_db), session_id)
    if not messages and not session_info:
        print(json.dumps({"status": "error", "message": "Session not found or no messages"}))
        sys.exit(1)

    # Step 2: Compress via Ollama
    summary = call_ollama_compress(messages, ollama_config, session_info)

    # Step 3: Store compression
    result = store_compression(str(state_db), session_id, summary)
    result["summary_preview"] = summary.get("summary", "")[:200] if "summary" in summary else None
    result["messages_count"] = len(messages)
    result["status"] = "success" if result.get("stored") else "error"

    print(json.dumps(result))


if __name__ == "__main__":
    main()
