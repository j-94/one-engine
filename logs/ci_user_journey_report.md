# CI User Journey Report

- Base URL: http://127.0.0.1:7777
- Branch ID: df9116d6-3dd3-4cd6-8fa3-db4977f9bbc7
- Engine log: /var/folders/pt/zp2r0_nd2yd92g_mp66td6_c0000gn/T/one_engine_ci_server.dZfkOxIehj

## Steps
1. Define persistent 'uppercase'
2. Approve pattern 'uppercase'
3. Call 'uppercase' with text='CI pass'

## Events Summary
- total: 16
- generated: 1
- approvals: 2
- calls: 2

## Status
- result: ✅ success

## Raw Responses (truncated)
### define ephemeral response
```
{"branch_id":"df9116d6-3dd3-4cd6-8fa3-db4977f9bbc7","effect":{"ApiCreated":{"api":{"name":"uppercase","description":"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.","parameters":[{"name":"text","param_type":null,"description":"parameter inferred from instruction"}],"logic":"Uppercase","persisted":false}}},"events":[{"Prompt":{"content":"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase."}},{"ParsedIntent":{"description":"CreateApi(CreateApiSpec { name: \"uppercase\", description: \"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\", parameters: [ApiParameterSpec { name: \"text\", param_type: None, description: Some(\"parameter inferred from instruction\") }], return_description: Some(\"it in uppercase.\"), persistence: Ephemeral, behavioral_hint: Custom(\"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\") })"}},{"ApiGenerated":{"name":"uppercase"}}]}

```
### call ephemeral response
```
{"branch_id":"df9116d6-3dd3-4cd6-8fa3-db4977f9bbc7","effect":{"ApiResponse":{"name":"uppercase","output":"CI PASS"}},"events":[{"Prompt":{"content":"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase."}},{"ParsedIntent":{"description":"CreateApi(CreateApiSpec { name: \"uppercase\", description: \"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\", parameters: [ApiParameterSpec { name: \"text\", param_type: None, description: Some(\"parameter inferred from instruction\") }], return_description: Some(\"it in uppercase.\"), persistence: Ephemeral, behavioral_hint: Custom(\"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\") })"}},{"ApiGenerated":{"name":"uppercase"}},{"Prompt":{"content":"Call the API 'uppercase' with text='CI pass'"}},{"ParsedIntent":{"description":"CallApi(CallApiSpec { name: \"uppercase\", arguments: {\"text\": \"CI pass\"} })"}},{"ApiCalled":{"name":"uppercase"}},{"ApiResponse":{"name":"uppercase","output":"CI PASS"}}]}

```
### define persistent response
```


```
### approve response
```
{"branch_id":"df9116d6-3dd3-4cd6-8fa3-db4977f9bbc7","effect":{"ApprovalRecorded":{"name":"uppercase"}},"events":[{"Prompt":{"content":"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase."}},{"ParsedIntent":{"description":"CreateApi(CreateApiSpec { name: \"uppercase\", description: \"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\", parameters: [ApiParameterSpec { name: \"text\", param_type: None, description: Some(\"parameter inferred from instruction\") }], return_description: Some(\"it in uppercase.\"), persistence: Ephemeral, behavioral_hint: Custom(\"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\") })"}},{"ApiGenerated":{"name":"uppercase"}},{"Prompt":{"content":"Call the API 'uppercase' with text='CI pass'"}},{"ParsedIntent":{"description":"CallApi(CallApiSpec { name: \"uppercase\", arguments: {\"text\": \"CI pass\"} })"}},{"ApiCalled":{"name":"uppercase"}},{"ApiResponse":{"name":"uppercase","output":"CI PASS"}},{"Prompt":{"content":"Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase."}},{"ParsedIntent":{"description":"CreateApi(CreateApiSpec { name: \"uppercase\", description: \"Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase.\", parameters: [], return_description: Some(\"it in uppercase.\"), persistence: Persist, behavioral_hint: Custom(\"Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase.\") })"}},{"Prompt":{"content":"Approve pattern 'uppercase'"}},{"ParsedIntent":{"description":"ApprovePattern { name: \"uppercase\" }"}},{"ParsedIntent":{"description":"approval: uppercase"}}]}

```
### call persisted response
```
{"branch_id":"df9116d6-3dd3-4cd6-8fa3-db4977f9bbc7","effect":{"ApiResponse":{"name":"uppercase","output":"CI PASS"}},"events":[{"Prompt":{"content":"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase."}},{"ParsedIntent":{"description":"CreateApi(CreateApiSpec { name: \"uppercase\", description: \"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\", parameters: [ApiParameterSpec { name: \"text\", param_type: None, description: Some(\"parameter inferred from instruction\") }], return_description: Some(\"it in uppercase.\"), persistence: Ephemeral, behavioral_hint: Custom(\"Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase.\") })"}},{"ApiGenerated":{"name":"uppercase"}},{"Prompt":{"content":"Call the API 'uppercase' with text='CI pass'"}},{"ParsedIntent":{"description":"CallApi(CallApiSpec { name: \"uppercase\", arguments: {\"text\": \"CI pass\"} })"}},{"ApiCalled":{"name":"uppercase"}},{"ApiResponse":{"name":"uppercase","output":"CI PASS"}},{"Prompt":{"content":"Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase."}},{"ParsedIntent":{"description":"CreateApi(CreateApiSpec { name: \"uppercase\", description: \"Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase.\", parameters: [], return_description: Some(\"it in uppercase.\"), persistence: Persist, behavioral_hint: Custom(\"Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase.\") })"}},{"Prompt":{"content":"Approve pattern 'uppercase'"}},{"ParsedIntent":{"description":"ApprovePattern { name: \"uppercase\" }"}},{"ParsedIntent":{"description":"approval: uppercase"}},{"Prompt":{"content":"Call the API 'uppercase' with text='CI pass'"}},{"ParsedIntent":{"description":"CallApi(CallApiSpec { name: \"uppercase\", arguments: {\"text\": \"CI pass\"} })"}},{"ApiCalled":{"name":"uppercase"}},{"ApiResponse":{"name":"uppercase","output":"CI PASS"}}]}

```
