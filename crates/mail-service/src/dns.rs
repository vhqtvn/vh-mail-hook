use anyhow::Result;
use common::AppError;
use trust_dns_resolver::TokioAsyncResolver;

#[async_trait::async_trait]
pub trait DnsResolver: Send + Sync {
    async fn mx_lookup(&self, domain: &str) -> Result<Vec<String>, AppError>;
}

pub struct TrustDnsResolver {
    resolver: TokioAsyncResolver,
}

impl TrustDnsResolver {
    pub async fn new() -> Result<Self> {
        let resolver = TokioAsyncResolver::tokio_from_system_conf()?;
        Ok(Self { resolver })
    }
}

#[async_trait::async_trait]
impl DnsResolver for TrustDnsResolver {
    async fn mx_lookup(&self, domain: &str) -> Result<Vec<String>, AppError> {
        let mx_lookup = self.resolver.mx_lookup(domain).await
            .map_err(|e| AppError::Mail(format!("Failed to lookup MX records: {}", e)))?;
        
        Ok(mx_lookup.iter().map(|mx| mx.exchange().to_string()).collect())
    }
}

#[cfg(any(test, feature = "test"))]
pub struct MockDnsResolver {
    mx_records: Vec<String>,
}

#[cfg(any(test, feature = "test"))]
impl MockDnsResolver {
    pub fn new(mx_records: Vec<String>) -> Self {
        Self { mx_records }
    }
}

#[cfg(any(test, feature = "test"))]
#[async_trait::async_trait]
impl DnsResolver for MockDnsResolver {
    async fn mx_lookup(&self, _domain: &str) -> Result<Vec<String>, AppError> {
        Ok(self.mx_records.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_resolver() {
        let mock_records = vec!["test-mx.example.com".to_string()];
        let resolver = MockDnsResolver::new(mock_records.clone());
        let result = resolver.mx_lookup("example.com").await.unwrap();
        assert_eq!(result, mock_records);
    }
} 