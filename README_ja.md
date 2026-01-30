# Orchix: Rust製 高性能エージェンティック・プロキシ

Orchixは、Rustで構築された軽量かつモジュール式の高性能な **Agentic Proxy** です。UNIXの哲学「1つのことを、うまくやる」に基づき、LLM、ツール、エージェント間の多機能な仲介役として、高度なオーケストレーション、セキュリティ、およびオブザーバビリティを提供します。

---

## 開発の目的

- **モジュール型アーキテクチャ**: ネットワーキング、ルーティング、ツール実行などを独立したコンポーネントとして構築。
- **エージェント・オーケストレーション**: タスク実行レイヤー（目標 -> 計画 -> 実行 -> 反省）をネイティブにサポート。
- **セキュリティとガードレール**: ツール呼び出しのインターセプト、個人情報（PII）のマスキング、RBACを内蔵。
- **圧倒的なパフォーマンス**: `tokio` による非同期I/Oを活用した低遅延プロキシ。
- **マルチプロトコル対応**: HTTP, WebSocket, gRPC, および MCP (Model Context Protocol) をサポート。

---

## 機能とロードマップ

### コア機能 (MVP)
- [x] **Networking**: `axum` を使用したHTTP/WS通信の基礎の実装済み。
- [x] **Config**: TOML/ENVによる柔軟な設定管理。
- [x] **Request Routing**: ルールベースのモデル選択とトラフィック制御。
- [x] **Tool Call Interception**: ツール実行要求に対するガードレール。
- [x] **Streaming Processing**: ストリーミングレスポンスのリアルタイム解析。
- [ ] **Security**: JWT/APIキー認証と監査ログ。
- [ ] **Caching**: レスポンス再利用のためのローカル/KVキャッシュ。
- [ ] **Cost Control**: トークン追跡と予算管理。
- [ ] **Observability**: メトリクス、ログ、トレース (Prometheus/OpenTelemetry)。

### エージェント機能
- [ ] **Task Execution Layer**: プロセス管理 (Goal -> Steps -> Reflect)。
- [ ] **Message Bus**: エージェント間通信と優先度付きキュー。
- [ ] **Tool Registry**: 動的なツール発見と機能マッピング。
- [ ] **Protocol Mediation**: A2A, MCP, OpenAPI 間のプロトコル仲介。

---

## インストールと使用方法

*(開発の進捗に合わせて順次公開予定)*

```bash
# ビルド例
cargo build --release
```

---

## ライセンス

MIT License (予定)
