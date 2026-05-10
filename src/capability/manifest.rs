//! Application Manifest
//!
//! Each application declares its capabilities at install time.
//! The kernel verifies M ⊆ K (manifest is subset of catalogue).

use super::{Capability, Catalogue};
use heapless::Vec;

/// Application manifest
///
/// Verified at install time. Cannot be modified at runtime.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Application identifier
    pub app_id: heapless::String<64>,
    /// Granted capabilities with rate limits
    pub capabilities: Vec<(Capability, u32), 4>, // max 4 capabilities
    /// Manifest version
    pub version: u32,
    /// HMAC-SHA256 signature (DC6)
    pub signature: [u8; 32],
}

/// Manifest builder
pub struct ManifestBuilder {
    app_id: Option<heapless::String<64>>,
    capabilities: Vec<(Capability, u32), 4>,
    version: u32,
}

impl ManifestBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            app_id: None,
            capabilities: Vec::new(),
            version: 1,
        }
    }

    /// Set application ID
    pub fn app_id(mut self, id: &str) -> Result<Self, ManifestError> {
        if id.len() > 64 {
            return Err(ManifestError::AppIdTooLong);
        }
        self.app_id = Some(heapless::String::from_str(id).unwrap());
        Ok(self)
    }

    /// Add capability with custom rate limit
    ///
    /// Rate limit must not exceed kernel maximum.
    pub fn capability_with_rate(
        mut self,
        cap: Capability,
        rate_hz: u32,
    ) -> Result<Self, ManifestError> {
        if !Catalogue::contains(&cap) {
            return Err(ManifestError::ProhibitedCapability);
        }
        if rate_hz > cap.max_rate_hz() {
            return Err(ManifestError::RateLimitExceeded);
        }
        self.capabilities.push((cap, rate_hz))
            .map_err(|_| ManifestError::TooManyCapabilities)?;
        Ok(self)
    }

    /// Add capability with default rate limit
    pub fn capability(self, cap: Capability) -> Result<Self, ManifestError> {
        self.capability_with_rate(cap, cap.max_rate_hz())
    }

    /// Build manifest
    pub fn build(self) -> Result<Manifest, ManifestError> {
        let app_id = self.app_id.ok_or(ManifestError::MissingAppId)?;
        Ok(Manifest {
            app_id,
            capabilities: self.capabilities,
            version: self.version,
            signature: [0u8; 32],
        })
    }
}

/// Manifest error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestError {
    AppIdTooLong,
    ProhibitedCapability,
    RateLimitExceeded,
    TooManyCapabilities,
    MissingAppId,
    InvalidSignature,
}

impl core::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AppIdTooLong => write!(f, "Application ID exceeds 64 characters"),
            Self::ProhibitedCapability => write!(f, "Capability not in catalogue (prohibited)"),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeds kernel maximum"),
            Self::TooManyCapabilities => write!(f, "Too many capabilities (max 4)"),
            Self::MissingAppId => write!(f, "Missing application ID"),
            Self::InvalidSignature => write!(f, "Invalid manifest signature"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest() {
        let manifest = ManifestBuilder::new()
            .app_id("com.example.cursor").unwrap()
            .capability(Capability::Navigation).unwrap()
            .build()
            .unwrap();
        assert_eq!(manifest.app_id, "com.example.cursor");
        assert_eq!(manifest.capabilities.len(), 1);
    }

    #[test]
    fn test_rate_limit_exceeded() {
        let result = ManifestBuilder::new()
            .app_id("com.example.cursor").unwrap()
            .capability_with_rate(Capability::Navigation, 100);
        assert!(matches!(result, Err(ManifestError::RateLimitExceeded)));
    }
}
