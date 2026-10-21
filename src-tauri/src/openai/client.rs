use super::{
    bounds::{calculate_period_and_today_bounds, current_unix_timestamp},
    types::{ModelUsage, OpenAiUsage},
};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const OPENAI_API_BASE: &str = "https://api.openai.com/v1/organization";
const OPENAI_USAGE_DAILY_PAGE_LIMIT: &str = "31";
const OPENAI_COSTS_DAILY_PAGE_LIMIT: &str = "180";
const MAX_OPENAI_PAGE_COUNT: usize = 10_000;

async fn api_error(response: reqwest::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or(body);
    format!("OpenAI Usage API 요청 실패 ({status}): {message}")
}

fn next_page_cursor(
    response: &Value,
    endpoint: &str,
    used_cursors: &mut HashSet<String>,
) -> Result<Option<String>, String> {
    if !response
        .get("has_more")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }

    let cursor = response
        .get("next_page")
        .and_then(Value::as_str)
        .filter(|cursor| !cursor.is_empty())
        .ok_or_else(|| format!("OpenAI {endpoint} 응답에 다음 페이지 커서가 없습니다."))?;
    if !used_cursors.insert(cursor.to_string()) {
        return Err(format!(
            "OpenAI {endpoint} 응답이 다음 페이지 커서를 반복했습니다."
        ));
    }

    Ok(Some(cursor.to_string()))
}

async fn fetch_openai_pages(
    client: &reqwest::Client,
    endpoint: &str,
    admin_key: &str,
    query: &[(&str, String)],
) -> Result<Vec<Value>, String> {
    let mut pages = Vec::new();
    let mut page_cursor: Option<String> = None;
    let mut used_cursors = HashSet::new();

    loop {
        if pages.len() >= MAX_OPENAI_PAGE_COUNT {
            return Err(format!(
                "OpenAI {endpoint} 페이지 수가 허용 한도를 초과했습니다."
            ));
        }

        let mut page_query = query.to_vec();
        if let Some(cursor) = &page_cursor {
            page_query.push(("page", cursor.clone()));
        }
        let response = client
            .get(format!("{OPENAI_API_BASE}/{endpoint}"))
            .bearer_auth(admin_key)
            .query(&page_query)
            .send()
            .await
            .map_err(|error| format!("OpenAI에 연결할 수 없습니다: {error}"))?;
        if !response.status().is_success() {
            return Err(api_error(response).await);
        }
        let page = response
            .json()
            .await
            .map_err(|error| format!("OpenAI {endpoint} 응답을 읽을 수 없습니다: {error}"))?;
        page_cursor = next_page_cursor(&page, endpoint, &mut used_cursors)?;
        pages.push(page);

        if page_cursor.is_none() {
            return Ok(pages);
        }
    }
}

fn aggregate_usage_pages(pages: &[Value]) -> (u64, u64, Vec<ModelUsage>) {
    let mut input_tokens = 0_u64;
    let mut output_tokens = 0_u64;
    let mut model_tokens = BTreeMap::<String, u64>::new();
    for page in pages {
        for bucket in page
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for result in bucket
                .get("results")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let input = result
                    .get("input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let output = result
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                input_tokens += input;
                output_tokens += output;
                let model = result
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or("Other");
                *model_tokens.entry(model.to_string()).or_default() += input + output;
            }
        }
    }

    let mut models = model_tokens
        .into_iter()
        .map(|(name, tokens)| ModelUsage { name, tokens })
        .collect::<Vec<_>>();
    models.sort_by_key(|model| std::cmp::Reverse(model.tokens));
    (input_tokens, output_tokens, models)
}

fn aggregate_cost_pages(pages: &[Value], today_start: i64) -> Result<(f64, f64, String), String> {
    let mut total_billed = 0.0_f64;
    let mut today_usage = 0.0_f64;
    let mut currency = None;
    for page in pages {
        for bucket in page
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let start_time = bucket
                .get("start_time")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let end_time = bucket
                .get("end_time")
                .and_then(Value::as_i64)
                .unwrap_or(start_time + 86400);
            let is_today = start_time >= today_start || end_time > today_start;
            for result in bucket
                .get("results")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let result_currency = result
                    .pointer("/amount/currency")
                    .and_then(Value::as_str)
                    .unwrap_or("usd")
                    .to_ascii_uppercase();
                if let Some(existing_currency) = &currency {
                    if existing_currency != &result_currency {
                        return Err(
                            "Costs API가 서로 다른 통화를 반환해 비용을 합산할 수 없습니다."
                                .to_string(),
                        );
                    }
                } else {
                    currency = Some(result_currency);
                }
                let amount = result
                    .pointer("/amount/value")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                total_billed += amount;
                if is_today {
                    today_usage += amount;
                }
            }
        }
    }

    Ok((
        total_billed,
        today_usage,
        currency.unwrap_or_else(|| "USD".to_string()),
    ))
}

pub(crate) async fn fetch_openai_usage_from_api(admin_key: &str) -> Result<OpenAiUsage, String> {
    if admin_key.trim().is_empty() {
        return Err("OpenAI 조직 관리자 API 키를 입력하세요.".to_string());
    }

    let now = current_unix_timestamp()?;
    let (period_start, today_start) = calculate_period_and_today_bounds(now);
    let client = reqwest::Client::new();
    let usage_pages = fetch_openai_pages(
        &client,
        "usage/completions",
        admin_key.trim(),
        &[
            ("start_time", period_start.to_string()),
            ("end_time", now.to_string()),
            ("bucket_width", "1d".to_string()),
            ("limit", OPENAI_USAGE_DAILY_PAGE_LIMIT.to_string()),
            ("group_by", "model".to_string()),
        ],
    )
    .await?;
    let costs_pages = fetch_openai_pages(
        &client,
        "costs",
        admin_key.trim(),
        &[
            ("start_time", period_start.to_string()),
            ("end_time", now.to_string()),
            ("bucket_width", "1d".to_string()),
            ("limit", OPENAI_COSTS_DAILY_PAGE_LIMIT.to_string()),
        ],
    )
    .await?;
    let (input_tokens, output_tokens, models) = aggregate_usage_pages(&usage_pages);
    let (total_billed, today_usage, currency) = aggregate_cost_pages(&costs_pages, today_start)?;

    Ok(OpenAiUsage {
        total_billed,
        today_usage,
        currency,
        input_tokens,
        output_tokens,
        models,
        period_start,
        period_end: now,
    })
}

#[cfg(test)]
mod tests {
    use super::{aggregate_cost_pages, aggregate_usage_pages, next_page_cursor};
    use serde_json::json;

    #[test]
    fn pagination_rejects_missing_and_repeated_continuation_cursors() {
        let mut cursors = std::collections::HashSet::new();
        assert!(next_page_cursor(
            &json!({ "has_more": true }),
            "usage/completions",
            &mut cursors
        )
        .is_err());
        let first_page = json!({ "has_more": true, "next_page": "cursor-one" });
        assert_eq!(
            next_page_cursor(&first_page, "usage/completions", &mut cursors).unwrap(),
            Some("cursor-one".to_string())
        );
        assert!(next_page_cursor(&first_page, "usage/completions", &mut cursors).is_err());
    }

    #[test]
    fn aggregation_combines_every_page_and_preserves_cost_currency() {
        let usage_pages = vec![
            json!({ "data": [{ "results": [{ "model": "gpt-test", "input_tokens": 10, "output_tokens": 5 }] }] }),
            json!({ "data": [{ "results": [{ "model": "gpt-test", "input_tokens": 7, "output_tokens": 3 }, { "model": "gpt-other", "input_tokens": 4, "output_tokens": 1 }] }] }),
        ];
        let (input_tokens, output_tokens, models) = aggregate_usage_pages(&usage_pages);
        assert_eq!((input_tokens, output_tokens), (21, 9));
        assert_eq!(
            (models[0].name.as_str(), models[0].tokens),
            ("gpt-test", 25)
        );
        assert_eq!(
            (models[1].name.as_str(), models[1].tokens),
            ("gpt-other", 5)
        );

        let cost_pages = vec![
            json!({ "data": [{ "start_time": 100, "end_time": 200, "results": [{ "amount": { "value": 0.5, "currency": "usd" } }] }] }),
            json!({ "data": [{ "start_time": 200, "end_time": 300, "results": [{ "amount": { "value": 1.0, "currency": "usd" } }] }] }),
        ];
        assert_eq!(
            aggregate_cost_pages(&cost_pages, 200).unwrap(),
            (1.5, 1.0, "USD".to_string())
        );
    }

    #[test]
    fn aggregation_rejects_currency_mismatch_across_pages() {
        let cost_pages = vec![
            json!({ "data": [{ "start_time": 100, "end_time": 200, "results": [{ "amount": { "value": 0.5, "currency": "usd" } }] }] }),
            json!({ "data": [{ "start_time": 200, "end_time": 300, "results": [{ "amount": { "value": 1.0, "currency": "eur" } }] }] }),
        ];
        assert!(aggregate_cost_pages(&cost_pages, 200).is_err());
    }
}
