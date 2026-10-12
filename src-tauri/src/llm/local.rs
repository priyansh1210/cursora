//! Local runtimes on this computer: the LM Studio probe (LLM-2.1) and the model size for this much RAM (LLM-2.4).
use super::net::{self, Policy, Request};
use std::time::Duration;

pub const LM_STUDIO: &str = "http://localhost:1234/v1";

/// The model ids of an OpenAI-compatible server on this computer, or `None` if nothing answers within 1.5 s.
pub async fn probe(base: &str) -> Option<Vec<String>> {
    let req = Request { url: format!("{base}/models").parse().ok()?, headers: vec![], body: None };
    let j = net::json(req, &Policy::default(), Duration::from_millis(1500)).await.ok()?;
    Some(super::openai::models(&j).into_iter().map(|m| m.id).collect())
}

/// The requirements table: total RAM → a model size that runs comfortably.
/// Rounded to whole GB, since a 16 GB computer reports a little less than 16 GB.
pub fn suggest_size(total_bytes: u64) -> &'static str {
    match (total_bytes + (1 << 29)) >> 30 {
        0..8 => "1–3B",
        8..16 => "3–4B",
        16..32 => "7–8B",
        _ => "up to 14B",
    }
}

#[cfg(windows)]
pub fn total_ram() -> Option<u64> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m) }.ok()?;
    Some(m.ullTotalPhys)
}

#[cfg(not(windows))]
pub fn total_ram() -> Option<u64> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = info.lines().find_map(|l| l.strip_prefix("MemTotal:"))?.trim().trim_end_matches("kB").trim().parse().ok()?;
    Some(kb * 1024)
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

    #[test]
    fn llm_2_4_the_suggested_size_follows_total_ram() {
        let gb = |n: u64| n << 30;
        assert_eq!(suggest_size(gb(4)), "1–3B");
        assert_eq!(suggest_size(gb(7)), "1–3B");
        assert_eq!(suggest_size(gb(16) - (300 << 20)), "7–8B", "16 GB reported as 15.7 GB");
        assert_eq!(suggest_size(gb(8)), "3–4B");
        assert_eq!(suggest_size(gb(15)), "3–4B");
        assert_eq!(suggest_size(gb(16)), "7–8B");
        assert_eq!(suggest_size(gb(31)), "7–8B");
        assert_eq!(suggest_size(gb(32)), "up to 14B");
        assert_eq!(suggest_size(gb(128)), "up to 14B");
        assert!(total_ram().is_some_and(|b| b > gb(1)));
    }
}
