use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::{Bytes, BytesMut};
use tracing::warn;
use serde_json::Value;
use crate::interception::Interceptor;
use std::sync::Arc;
use axum::response::sse::Event;

/// ストリーミングレスポンスを解析するためのラッパー
pub struct StreamingAnalyzer<S> {
    inner: S,
    interceptor: Arc<Interceptor>,
    buffer: BytesMut,
    pending_events: std::collections::VecDeque<Result<Event, axum::Error>>,
    full_response_buffer: BytesMut,
    cache_info: Option<(crate::cache::OrchixCache, crate::cache::CacheKey)>,
}

impl<S> StreamingAnalyzer<S> {
    pub fn new(
        inner: S, 
        interceptor: Arc<Interceptor>, 
        cache_info: Option<(crate::cache::OrchixCache, crate::cache::CacheKey)>
    ) -> Self {
        Self { 
            inner, 
            interceptor,
            buffer: BytesMut::new(),
            pending_events: std::collections::VecDeque::new(),
            full_response_buffer: BytesMut::new(),
            cache_info,
        }
    }

    pub fn get_full_response(&self) -> Bytes {
        self.full_response_buffer.clone().freeze()
    }

    /// 改行区切りで SSE 行を抽出して解析する
    fn process_buffer(&mut self) {
        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let line_bytes = self.buffer.split_to(pos + 1);
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            if line.starts_with("data: ") {
                let data = &line[6..];
                
                // 特定のデータを解析
                if data != "[DONE]" {
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Err(msg) = self.content_interception(&json) {
                            self.pending_events.push_back(Err(axum::Error::new(msg)));
                            return;
                        }
                    }
                }

                // Event として再構築して追加
                self.pending_events.push_back(Ok(axum::response::sse::Event::default().data(data)));
            }
        }
    }

    /// ストリーム内の JSON チャンクを解析し、ポリシー違反がないかチェックする
    fn content_interception(&self, json: &Value) -> Result<(), String> {
        // choices[0].delta.tool_calls などを想定
        if let Some(choices) = json.get("choices").and_then(|v| v.as_array()) {
            for choice in choices {
                if let Some(delta) = choice.get("delta") {
                    // delta 内の tool_calls をチェック
                    if let Err(msg) = self.interceptor.validate_tools(delta) {
                        warn!("Forbidden tool detected in stream: {}", msg);
                        return Err(msg);
                    }
                }
            }
        }
        Ok(())
    }
}

impl<S> Stream for StreamingAnalyzer<S>
where
    S: Stream<Item = Result<Bytes, axum::Error>> + Unpin,
{
    type Item = Result<Event, axum::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 保留中のイベントがあればそれを優先的に返す
        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(Some(event));
        }

        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                self.buffer.extend_from_slice(&bytes);
                self.full_response_buffer.extend_from_slice(&bytes);
                self.process_buffer();
                
                // バッファを処理した後にイベントがあれば返す
                if let Some(event) = self.pending_events.pop_front() {
                    Poll::Ready(Some(event))
                } else {
                    // まだ行が完成していなければ次のチャンクを待つ
                    // Note: 本来はここで再度 poll したいが、再帰を避けるため
                    // Context を使って再度 wake する必要がある
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => {
                // ストリーム終了時に残りのバッファを処理
                self.process_buffer();
                
                // キャッシュ情報があれば保存
                if let Some((cache, key)) = self.cache_info.take() {
                    let body = self.full_response_buffer.clone().freeze();
                    tokio::spawn(async move {
                        let mut headers = std::collections::HashMap::new();
                        headers.insert("content-type".to_string(), "text/event-stream".to_string());
                        cache.set(key, crate::cache::CachedResponse {
                            status: 200,
                            headers,
                            body,
                        }).await;
                    });
                }

                if let Some(event) = self.pending_events.pop_front() {
                    Poll::Ready(Some(event))
                } else {
                    Poll::Ready(None)
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
