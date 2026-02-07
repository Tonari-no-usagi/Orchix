# Orchix: High-Performance Agentic Proxy in Rust

Orchix is a lightweight, modular, and high-performance **Agentic Proxy** built with Rust. Following the Unix philosophy of "doing one thing and doing it well," Orchix acts as a versatile intermediary between LLMs, Tools, and Agents, providing advanced orchestration, security, and observability.

[日本語版はこちら (README_ja.md)](./README_ja.md)

---

## Key Objectives

- **Modular Architecture**: Built as independent components for networking, routing, and tool execution.
- **Agentic Orchestration**: Native support for Task Execution Layers (Goal -> Plan -> Execute -> Reflect).
- **Security & Guardrails**: Built-in tool call interception, PII masking, and RBAC.
- **High Performance**: Leveraging `tokio` for asynchronous I/O and low-latency proxying.
- **Multi-Protocol**: Support for HTTP, WebSocket, gRPC, and MCP (Model Context Protocol).

---

## Features & Roadmap

### Core MVP
- [x] **Networking**: HTTP/WS communication foundation using `axum`.
- [x] **Config**: Flexible configuration management (TOML/ENV).
- [x] **Request Routing**: Rule-based model selection and traffic control.
- [x] **Tool Call Interception**: Guardrails for tool execution requests.
- [x] **Streaming Processing**: Real-time analysis of streaming responses.
- [x] **Security**: JWT/API Key authentication and audit logging.
- [x] **Caching**: Local/KV caching for response reuse.
- [ ] **Cost Control**: Token tracking and budget management.
- [ ] **Observability**: Metrics, logs, and tracing (Prometheus/OpenTelemetry).

### Agentic Features
- [ ] **Task Execution Layer**: Process management (Goal -> Steps -> Reflect).
- [ ] **Message Bus**: Inter-agent communication and priority queuing.
- [ ] **Tool Registry**: Dynamic tool discovery and capability mapping.
- [ ] **Protocol Mediation**: Bridging between A2A, MCP, and OpenAPI.

---

## Installation & Usage

*(Coming soon as the project reaches its first release version)*

```bash
# Example: Build the project
cargo build --release
```

---

## License

MIT License (Tentative)
