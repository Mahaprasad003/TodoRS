use anyhow::Result;
use reqwest::Client;

/// A client for syncing operations with the Supabase backend.
///
/// Handles authentication and provides methods for uploading and
/// downloading operations via the deployed edge functions.
pub struct SyncClient {
    client: Client,
    base_url: String,
    api_key: String,
    access_token: Option<String>,
}

impl SyncClient {
    /// Create a new SyncClient.
    ///
    /// * `base_url` — The Supabase project URL (e.g. `https://xxxxx.supabase.co`).
    /// * `api_key` — The Supabase anon/public key.
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            access_token: None,
        }
    }

    /// Authenticate with email and password.
    ///
    /// Stores the access token internally for subsequent API calls.
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

        Ok(self.access_token.clone().unwrap_or_default())
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

    /// Download operations with sequence greater than `since_seq`.
    ///
    /// Requires a valid access token (call `login` first).
    pub async fn get_operations(
        &self,
        since_seq: i64,
    ) -> Result<Vec<super::operations::Operation>> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let response = self
            .client
            .get(format!(
                "{}/functions/v1/get-operations?since={}",
                self.base_url, since_seq
            ))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let operations = data
            .get("operations")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Deserialize operations
        let ops: Vec<super::operations::Operation> = operations
            .into_iter()
            .filter_map(|op| serde_json::from_value(op).ok())
            .collect();

        Ok(ops)
    }
}
