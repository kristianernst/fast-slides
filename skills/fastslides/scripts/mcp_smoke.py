#!/usr/bin/env python3
"""Smoke test for FastSlides embedded MCP server over localhost."""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from typing import Any


DEFAULT_MCP_URL = "http://127.0.0.1:38474/mcp"
DEFAULT_PROTOCOL_VERSION = "2025-11-25"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--url",
        default=os.environ.get("FASTSLIDES_MCP_URL", DEFAULT_MCP_URL),
        help=f"MCP endpoint URL (default: {DEFAULT_MCP_URL})",
    )
    parser.add_argument(
        "--protocol-version",
        default=DEFAULT_PROTOCOL_VERSION,
        help=f"MCP protocol version for initialize (default: {DEFAULT_PROTOCOL_VERSION})",
    )
    parser.add_argument(
        "--path",
        help="Optional project path used to call open_project + validate_project tools.",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=10.0,
        help="HTTP timeout in seconds (default: 10)",
    )
    return parser.parse_args()


def extract_jsonrpc(body: str) -> dict[str, Any]:
    body = body.strip()
    if not body:
        raise RuntimeError("Empty MCP response body.")

    if body.startswith("{"):
        payload = json.loads(body)
        if isinstance(payload, dict):
            return payload
        raise RuntimeError(f"Expected JSON object, got: {type(payload)}")

    # Streamable HTTP may return SSE payloads. Extract first JSON data event.
    chunks: list[str] = []
    current: list[str] = []
    for line in body.splitlines():
        if line.startswith("data:"):
            current.append(line[5:].lstrip())
            continue
        if not line.strip() and current:
            chunks.append("\n".join(current))
            current = []
    if current:
        chunks.append("\n".join(current))

    for chunk in chunks:
        candidate = chunk.strip()
        if not candidate:
            continue
        try:
            parsed = json.loads(candidate)
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict):
            return parsed

    raise RuntimeError(f"Unable to parse JSON-RPC payload from response: {body[:400]}")


def _build_headers(
    *,
    session_id: str | None = None,
    protocol_version: str | None = None,
) -> dict[str, str]:
    headers = {
        "content-type": "application/json",
        "accept": "application/json, text/event-stream",
    }
    if session_id:
        headers["MCP-Session-Id"] = session_id
    if protocol_version:
        headers["MCP-Protocol-Version"] = protocol_version
    return headers


def post_json(
    *,
    url: str,
    payload: dict[str, Any],
    timeout: float,
    session_id: str | None = None,
    protocol_version: str | None = None,
) -> tuple[dict[str, Any], str | None]:
    headers = _build_headers(
        session_id=session_id,
        protocol_version=protocol_version,
    )
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(url=url, data=data, headers=headers, method="POST")

    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            response_body = response.read().decode("utf-8", errors="replace")
            parsed = extract_jsonrpc(response_body)
            return parsed, response.headers.get("MCP-Session-Id")
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code} from {url}: {body[:400]}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Unable to reach MCP endpoint at {url}: {error}") from error


def post_notification(
    *,
    url: str,
    payload: dict[str, Any],
    timeout: float,
    session_id: str | None = None,
    protocol_version: str | None = None,
) -> None:
    headers = _build_headers(
        session_id=session_id,
        protocol_version=protocol_version,
    )
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(url=url, data=data, headers=headers, method="POST")

    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            response.read()
            if response.status not in (200, 202, 204):
                raise RuntimeError(
                    f"Unexpected status for notification {payload.get('method')}: {response.status}"
                )
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(
            f"HTTP {error.code} from {url} for notification {payload.get('method')}: {body[:400]}"
        ) from error
    except urllib.error.URLError as error:
        raise RuntimeError(
            f"Unable to reach MCP endpoint at {url} for notification {payload.get('method')}: {error}"
        ) from error


def delete_session(
    *,
    url: str,
    timeout: float,
    session_id: str,
    protocol_version: str | None = None,
) -> bool:
    headers = _build_headers(session_id=session_id, protocol_version=protocol_version)
    request = urllib.request.Request(url=url, headers=headers, method="DELETE")

    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            response.read()
            return response.status in (200, 202, 204)
    except urllib.error.HTTPError as error:
        if error.code in (404, 405):
            return False
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code} from {url} during session close: {body[:400]}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Failed to close MCP session at {url}: {error}") from error


def require_success(payload: dict[str, Any], label: str) -> dict[str, Any]:
    if "error" in payload:
        raise RuntimeError(f"{label} returned error: {json.dumps(payload['error'])}")
    result = payload.get("result")
    if not isinstance(result, dict):
        raise RuntimeError(f"{label} missing JSON-RPC result object: {json.dumps(payload)}")
    return result


def main() -> int:
    args = parse_args()

    session_id: str | None = None
    negotiated_protocol = args.protocol_version
    try:
        initialize_payload = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": args.protocol_version,
                "capabilities": {},
                "clientInfo": {
                    "name": "fastslides-mcp-smoke",
                    "version": "0.1.0",
                },
            },
        }
        initialize_response, session_id = post_json(
            url=args.url,
            payload=initialize_payload,
            timeout=args.timeout,
            session_id=None,
            protocol_version=args.protocol_version,
        )
        initialize_result = require_success(initialize_response, "initialize")
        server_info = initialize_result.get("serverInfo") or initialize_result.get("server_info")
        if not isinstance(server_info, dict):
            raise RuntimeError(f"initialize result missing serverInfo: {json.dumps(initialize_result)}")

        negotiated_protocol = (
            initialize_result.get("protocolVersion")
            or initialize_result.get("protocol_version")
            or args.protocol_version
        )
        if not isinstance(negotiated_protocol, str):
            negotiated_protocol = args.protocol_version

        print(
            f"[MCP] initialize ok: server={server_info.get('name')} "
            f"session={session_id or 'none'} protocol={negotiated_protocol}"
        )

        post_notification(
            url=args.url,
            payload={
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {},
            },
            timeout=args.timeout,
            session_id=session_id,
            protocol_version=negotiated_protocol,
        )
        print("[MCP] notifications/initialized ok")

        list_tools_payload = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {},
        }
        list_response, _ = post_json(
            url=args.url,
            payload=list_tools_payload,
            timeout=args.timeout,
            session_id=session_id,
            protocol_version=negotiated_protocol,
        )
        list_result = require_success(list_response, "tools/list")
        tools = list_result.get("tools", [])
        if not isinstance(tools, list):
            raise RuntimeError(f"tools/list returned invalid tools payload: {json.dumps(list_result)}")

        tool_names = sorted(
            tool.get("name") for tool in tools if isinstance(tool, dict) and isinstance(tool.get("name"), str)
        )
        print(f"[MCP] tools/list ok: {', '.join(tool_names)}")

        call_health_payload = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "health",
                "arguments": {},
            },
        }
        call_health_response, _ = post_json(
            url=args.url,
            payload=call_health_payload,
            timeout=args.timeout,
            session_id=session_id,
            protocol_version=negotiated_protocol,
        )
        call_health_result = require_success(call_health_response, "tools/call health")
        if call_health_result.get("isError"):
            raise RuntimeError(f"tools/call health returned isError=true: {json.dumps(call_health_result)}")
        print("[MCP] tools/call health ok")

        if args.path:
            for req_id, tool_name in ((4, "open_project"), (5, "validate_project")):
                payload = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "method": "tools/call",
                    "params": {
                        "name": tool_name,
                        "arguments": {"path": args.path},
                    },
                }
                response, _ = post_json(
                    url=args.url,
                    payload=payload,
                    timeout=args.timeout,
                    session_id=session_id,
                    protocol_version=negotiated_protocol,
                )
                result = require_success(response, f"tools/call {tool_name}")
                if result.get("isError"):
                    raise RuntimeError(
                        f"tools/call {tool_name} returned isError=true: {json.dumps(result)}"
                    )
                print(f"[MCP] tools/call {tool_name} ok")

        print("[MCP] smoke test passed")
        return 0
    finally:
        if session_id:
            try:
                closed = delete_session(
                    url=args.url,
                    timeout=args.timeout,
                    session_id=session_id,
                    protocol_version=negotiated_protocol,
                )
                if closed:
                    print("[MCP] session closed")
            except Exception as error:  # noqa: BLE001
                print(f"[MCP][WARN] Failed to close session cleanly: {error}", file=sys.stderr)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # noqa: BLE001
        print(f"[MCP][ERROR] {error}", file=sys.stderr)
        raise SystemExit(1)
