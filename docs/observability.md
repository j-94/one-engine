# One Engine Observable Run Log

This document captures an end-to-end, observable demonstration of One Engine learning through conversation, respecting safety boundaries, and producing a reusable cognitive asset within a branch.

- Engine URL: http://127.0.0.1:7777
- Branch label: learning-loop
- Branch ID: fa978544-b662-40d7-9d38-1a7c5a72dd69
- Artifacts directory: out_one_engine/

## 1) Health and Version

```json path=/Users/imac/Desktop/one-engine/out_one_engine/healthz.json start=1
{
  "ok": true,
  "consciousness_active": true,
  "pattern_db_size": 0
}
```

```json path=/Users/imac/Desktop/one-engine/out_one_engine/version.json start=1
{
  "version": "0.1.0",
  "build_token": "dev-1760357480",
  "crystallized_patterns": 0
}
```

## 2) Conversational learning (ephemeral → persistent + approval → reuse)

- Define 'echo' (ephemeral):
```json path=/Users/imac/Desktop/one-engine/out_one_engine/01_define_echo_ephemeral.json start=1
{}
```

- Call 'echo' (Hello, Loop):
```json path=/Users/imac/Desktop/one-engine/out_one_engine/02_call_echo_ephemeral.json start=1
{}
```

- Approve persistent 'echo':
```json path=/Users/imac/Desktop/one-engine/out_one_engine/04_approve_echo.json start=1
{}
```

- Call 'echo' again (Hello Again):
```json path=/Users/imac/Desktop/one-engine/out_one_engine/05_call_echo_persisted.json start=1
{}
```

- Call after UTIR checks (After UTIR, still works):
```json path=/Users/imac/Desktop/one-engine/out_one-engine/06_call_echo_after_utir.json start=1
{}
```

- Events tail (first phase):
```json path=/Users/imac/Desktop/one-engine/out_one_engine/events_tail.json start=1
{}
```

## 3) Safety boundaries via UTIR

- Blocked shell (rm -rf /):
```json path=/Users/imac/Desktop/one-engine/out_one_engine/utir_blocked_shell.json start=1
{}
```

- Disallowed domain (example.com):
```json path=/Users/imac/Desktop/one-engine/out_one_engine/utir_disallowed_domain.json start=1
{}
```

- Allowed domain (healthz on localhost):
```json path=/Users/imac/Desktop/one-engine/out_one_engine/utir_allowed_domain.json start=1
{}
```

## 4) Additional nudges (slugify, reverse, counter)

- Tail after additional nudges:
```json path=/Users/imac/Desktop/one-engine/out_one_engine/next_nudges_events_tail.json start=1
{}
```

## 5) Git history (observability)

```text path=/Users/imac/Desktop/one-engine/out_one_engine/git_log.txt start=1
*
```

Notes:
- The current engine maps simple definitions to Echo or Custom behaviors. Approval events are recorded for governance. Effects for Custom logic may be placeholders until logic is implemented.
- Branch-scoped persistence is demonstrated within the session branch; cross-restart crystallization can be wired via MemorySystem for “immortal” tools.
