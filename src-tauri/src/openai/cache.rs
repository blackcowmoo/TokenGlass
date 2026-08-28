use super::{
    bounds::current_unix_timestamp,
    client::fetch_openai_usage_from_api,
    types::{OpenAiUsage, OpenAiUsageSnapshot},
};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

const OPENAI_USAGE_CACHE_TTL_SECONDS: i64 = 5 * 60;

#[derive(Clone)]
struct CachedOpenAiUsage {
    key_fingerprint: String,
    usage: OpenAiUsage,
    fetched_at: i64,
    generation: u64,
}

#[derive(Default)]
pub struct OpenAiUsageState {
    cache: Mutex<Option<CachedOpenAiUsage>>,
    refresh_gate: Mutex<()>,
}

fn openai_key_fingerprint(admin_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(admin_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn cache_is_fresh(entry: &CachedOpenAiUsage, now: i64) -> bool {
    now.saturating_sub(entry.fetched_at) < OPENAI_USAGE_CACHE_TTL_SECONDS
}

fn cache_matches_key(entry: &CachedOpenAiUsage, key_fingerprint: &str) -> bool {
    entry.key_fingerprint == key_fingerprint
}

fn should_return_fresh_cache(
    entry: &CachedOpenAiUsage,
    key_fingerprint: &str,
    force_refresh: bool,
    now: i64,
) -> bool {
    cache_matches_key(entry, key_fingerprint) && !force_refresh && cache_is_fresh(entry, now)
}

fn refresh_completed_while_waiting(
    entry: &CachedOpenAiUsage,
    key_fingerprint: &str,
    observed_generation: Option<u64>,
) -> bool {
    cache_matches_key(entry, key_fingerprint)
        && observed_generation
            .map(|generation| entry.generation > generation)
            .unwrap_or(false)
}

fn usage_snapshot(
    entry: &CachedOpenAiUsage,
    source: &str,
    stale: bool,
    refresh_error: Option<String>,
) -> OpenAiUsageSnapshot {
    OpenAiUsageSnapshot {
        usage: entry.usage.clone(),
        fetched_at: entry.fetched_at,
        source: source.to_string(),
        stale,
        refresh_error,
    }
}

pub async fn fetch_usage(
    admin_key: &str,
    force_refresh: bool,
    usage_state: &OpenAiUsageState,
) -> Result<OpenAiUsageSnapshot, String> {
    if admin_key.is_empty() {
        return Err("OpenAI 조직 관리자 API 키를 입력하세요.".to_string());
    }

    let key_fingerprint = openai_key_fingerprint(admin_key);
    let now = current_unix_timestamp()?;
    let observed_generation = {
        let cache = usage_state.cache.lock().await;
        match cache.as_ref() {
            Some(entry) if cache_matches_key(entry, &key_fingerprint) => {
                if should_return_fresh_cache(entry, &key_fingerprint, force_refresh, now) {
                    return Ok(usage_snapshot(entry, "cache", false, None));
                }
                Some(entry.generation)
            }
            _ => None,
        }
    };

    let _refresh_guard = usage_state.refresh_gate.lock().await;
    let now = current_unix_timestamp()?;
    {
        let cache = usage_state.cache.lock().await;
        if let Some(entry) = cache.as_ref() {
            let refreshed_while_waiting =
                refresh_completed_while_waiting(entry, &key_fingerprint, observed_generation);
            if should_return_fresh_cache(entry, &key_fingerprint, force_refresh, now)
                || refreshed_while_waiting
            {
                return Ok(usage_snapshot(entry, "cache", false, None));
            }
        }
    }

    match fetch_openai_usage_from_api(admin_key).await {
        Ok(usage) => {
            let fetched_at = current_unix_timestamp()?;
            let mut cache = usage_state.cache.lock().await;
            let generation = cache
                .as_ref()
                .map(|entry| entry.generation.saturating_add(1))
                .unwrap_or(1);
            let entry = CachedOpenAiUsage {
                key_fingerprint,
                usage,
                fetched_at,
                generation,
            };
            let snapshot = usage_snapshot(&entry, "network", false, None);
            *cache = Some(entry);
            Ok(snapshot)
        }
        Err(error) => {
            let cache = usage_state.cache.lock().await;
            if let Some(entry) = cache
                .as_ref()
                .filter(|entry| cache_matches_key(entry, &key_fingerprint))
            {
                Ok(usage_snapshot(entry, "cache", true, Some(error)))
            } else {
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cache_is_fresh, cache_matches_key, openai_key_fingerprint, refresh_completed_while_waiting,
        should_return_fresh_cache, usage_snapshot, CachedOpenAiUsage, OpenAiUsage,
        OpenAiUsageState, OPENAI_USAGE_CACHE_TTL_SECONDS,
    };
    use crate::openai::types::ModelUsage;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn cached_usage(key: &str, fetched_at: i64, generation: u64) -> CachedOpenAiUsage {
        CachedOpenAiUsage {
            key_fingerprint: openai_key_fingerprint(key),
            usage: OpenAiUsage {
                total_billed: 1.25,
                today_usage: 0.5,
                currency: "USD".to_string(),
                input_tokens: 10,
                output_tokens: 20,
                models: vec![ModelUsage {
                    name: "gpt-test".to_string(),
                    tokens: 30,
                }],
                period_start: 100,
                period_end: 200,
            },
            fetched_at,
            generation,
        }
    }

    #[test]
    fn cache_respects_ttl_force_refresh_and_key_isolation() {
        let key = "sk-admin-one";
        let entry = cached_usage(key, 1_000, 7);
        let fingerprint = openai_key_fingerprint(key);
        assert!(cache_is_fresh(
            &entry,
            1_000 + OPENAI_USAGE_CACHE_TTL_SECONDS - 1
        ));
        assert!(!cache_is_fresh(
            &entry,
            1_000 + OPENAI_USAGE_CACHE_TTL_SECONDS
        ));
        assert!(should_return_fresh_cache(
            &entry,
            &fingerprint,
            false,
            1_001
        ));
        assert!(!should_return_fresh_cache(
            &entry,
            &fingerprint,
            true,
            1_001
        ));
        assert!(!cache_matches_key(
            &entry,
            &openai_key_fingerprint("sk-admin-two")
        ));
    }

    #[test]
    fn stale_snapshot_keeps_last_successful_data() {
        let entry = cached_usage("sk-admin-one", 1_000, 7);
        let snapshot = usage_snapshot(
            &entry,
            "cache",
            true,
            Some("network unavailable".to_string()),
        );
        assert!(snapshot.stale);
        assert_eq!(snapshot.fetched_at, 1_000);
        assert_eq!(snapshot.usage.total_billed, 1.25);
        assert_eq!(
            snapshot.refresh_error.as_deref(),
            Some("network unavailable")
        );
    }

    #[tokio::test]
    async fn concurrent_requests_reuse_one_completed_refresh_generation() {
        let state = Arc::new(OpenAiUsageState::default());
        let key = "sk-admin-one";
        let fingerprint = openai_key_fingerprint(key);
        {
            let mut cache = state.cache.lock().await;
            *cache = Some(cached_usage(key, 1_000, 1));
        }

        let refresh_count = Arc::new(AtomicUsize::new(0));
        let refresh_gate = state.refresh_gate.lock().await;
        refresh_count.fetch_add(1, Ordering::SeqCst);
        {
            let mut cache = state.cache.lock().await;
            *cache = Some(cached_usage(key, 1_500, 2));
        }

        let waiting_state = Arc::clone(&state);
        let waiting_fingerprint = fingerprint.clone();
        let waiting_request = tokio::spawn(async move {
            let _gate = waiting_state.refresh_gate.lock().await;
            let cache = waiting_state.cache.lock().await;
            let entry = cache.as_ref().expect("refresh should populate the cache");
            refresh_completed_while_waiting(entry, &waiting_fingerprint, Some(1))
        });

        drop(refresh_gate);
        assert!(waiting_request
            .await
            .expect("waiting request should complete"));
        assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    }
}
