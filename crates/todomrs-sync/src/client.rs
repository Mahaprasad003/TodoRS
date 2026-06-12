use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use uuid::Uuid;

/// A client for syncing operations with the Supabase backend.
///
/// Handles authentication and provides methods for uploading and
/// downloading operations via the deployed edge functions.
pub struct SyncClient {
    client: Client,
    base_url: String,
    api_key: String,
    access_token: Option<String>,
    supabase_user_id: Option<Uuid>,
}

impl SyncClient {
    /// Create a new SyncClient.
    ///
    /// * `base_url` — The Supabase project URL (e.g. `https://xxxxx.supabase.co`).
    /// * `api_key` — The Supabase anon/public key.
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest Client::builder() never fails"),
            base_url,
            api_key,
            access_token: None,
            supabase_user_id: None,
        }
    }

    /// Returns `true` if the client has a valid access token.
    pub fn is_authenticated(&self) -> bool {
        self.access_token.is_some()
    }

    /// Authenticate with email and password.
    ///
    /// Stores the access token and supabase user ID internally for subsequent API calls.
    /// Returns the access token string.
    pub async fn login(&mut self, email: &str, password: &str) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/auth/v1/token?grant_type=password", self.base_url))
            .header("apikey", &self.api_key)
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        self.access_token = data
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Capture the real Supabase auth user ID from the login response
        if let Some(user_id_str) = data
            .get("user")
            .and_then(|u| u.get("id"))
            .and_then(|v| v.as_str())
        {
            if let Ok(uid) = Uuid::parse_str(user_id_str) {
                self.supabase_user_id = Some(uid);
            }
        }

        Ok(self.access_token.clone().unwrap_or_default())
    }

    /// Returns the authenticated Supabase user ID, if available.
    pub fn supabase_user_id(&self) -> Option<Uuid> {
        self.supabase_user_id
    }

    /// Upload a batch of operations to the backend.
    ///
    /// Requires a valid access token (call `login` first).
    pub async fn upload_operations(
        &self,
        operations: Vec<super::operations::Operation>,
    ) -> Result<()> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        self.client
            .post(format!(
                "{}/functions/v1/upload-operations",
                self.base_url
            ))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "operations": operations }))
            .send()
            .await?;

        Ok(())
    }

    /// Download operations created after `since_time`.
    ///
    /// Uses `created_at` (global timestamp) for filtering instead of
    /// per-device sequence numbers, making it safe across multiple devices.
    ///
    /// Requires a valid access token (call `login` first).
    pub async fn get_operations(
        &self,
        since_time: DateTime<Utc>,
    ) -> Result<Vec<super::operations::Operation>> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let since_str = since_time.to_rfc3339();

        let response = self
            .client
            .get(format!("{}/functions/v1/get-operations", self.base_url))
            .query(&[("since", since_str.as_str())])
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "get-operations failed {}: {}",
                status,
                body
            ));
        }

        let data: serde_json::Value = serde_json::from_str(&body)?;
        let operations = data
            .get("operations")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Deserialize operations, silently skipping any that don't match
        // the current schema (legacy operations with uppercase casing or
        // flat payloads are known to exist in the database).
        let ops: Vec<super::operations::Operation> = operations
            .into_iter()
            .filter_map(|op| serde_json::from_value(op).ok())
            .collect();

        Ok(ops)
    }
}


