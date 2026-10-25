//! Local runtimes on this computer: the LM Studio probe (LLM-2.1).
use super::net::{self, Policy, Request};
use std::time::Duration;

pub const LM_STUDIO: &str = "http://localhost:1234/v1";

/// The model ids of an OpenAI-compatible server on this computer, or `None` if nothing answers within 1.5 s.
pub async fn probe(base: &str) -> Option<Vec<String>> {
    let req = Request { url: format!("{base}/models").parse().ok()?, headers: vec![], body: None };
    let j = net::json(req, &Policy::default(), Duration::from_millis(1500)).await.ok()?;
    Some(super::openai::models(&j).into_iter().map(|m| m.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::net::tests::{serve, Step};

    #[test]
    fn llm_2_1_the_probe_lists_a_running_server_and_ignores_a_missing_one() {
        let (port, _) = serve(vec![Step::Write(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"data\":[{\"id\":\"qwen2.5-7b-instruct\",\"object\":\"model\"},{\"id\":\"text-embedding-nomic\",\"object\":\"model\"}],\"object\":\"list\"}",
        )]);
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        assert_eq!(rt.block_on(probe(&format!("http://127.0.0.1:{port}/v1"))).unwrap(), ["qwen2.5-7b-instruct", "text-embedding-nomic"]);
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port(); // nothing listens once dropped
        assert_eq!(rt.block_on(probe(&format!("http://127.0.0.1:{free}/v1"))), None);
    }
}
