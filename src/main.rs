use std::{env, error::Error, ffi::OsString, process};

use chrono::NaiveDate;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
// This lets us write `#[derive(Deserialize)]`.
use serde::Deserialize;

mod date_format {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%m/%d/%Y";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

// We don't need to derive `Debug` (which doesn't require Serde), but it's a
// good habit to do it for all your types.
//
// Notice that the field names in this struct are NOT in the same order as
// the fields in the CSV data!
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Transaction {
    #[serde(rename = "Transaction Date", with = "date_format")]
    transaction_date: NaiveDate,
    amount: f64,
    #[serde(rename = "Post Date", with = "date_format")]
    post_date: NaiveDate,
    description: String,
    category: String,
    #[serde(rename = "Type")]
    transaction_type: String,
    memo: String,
}

#[derive(Debug, Deserialize)]
struct BudgetsResponse {
    data: BudgetsData,
}

#[derive(Debug, Deserialize)]
struct BudgetsData {
    budgets: Vec<Budget>,
}

#[derive(Debug, Deserialize)]
struct Budget {
    id: String,
    name: String,
}

fn get_ynab_api_key() -> Result<String, String> {
    match env::var("YNAB_API_KEY") {
        Ok(val) => Ok(val),
        Err(_) => Err("YNAB_API_KEY environment variable must be set".to_string()),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let mut reader = csv::Reader::from_path(file_path)?;
    for result in reader.deserialize() {
        let transaction: Transaction = result?;
        println!("{:?}", transaction);
    }

    let api_key = get_ynab_api_key()?;
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}"))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::blocking::Client::new();
    let budgets_response: BudgetsResponse = client
        .get("https://api.ynab.com/v1/budgets")
        .headers(headers)
        .send()?
        .json()?;

    for budget in &budgets_response.data.budgets {
        println!("{} ({})", budget.name, budget.id);
    }

    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
