//! Borrowed immutable observable policies, with explicitly bounded duplicate checks.
use super::{budget::Budget, VerificationError};

/// One required observable in a frozen experiment profile.
/// The benchmark defines the meaning, units and complete mandatory key inventory.
#[derive(Debug, Clone, Copy)]
pub struct ObservablePolicy {
    /// Stable key; changing the required inventory changes the experiment's policy identity.
    pub key: u32,
    /// Total tolerance and all channel allocations in this observable's units.
    pub budget: Budget,
}

/// Nonempty policies with unique keys. The borrow prevents mutation during review.
#[derive(Debug, Clone, Copy)]
pub struct Policies<'a> {
    entries: &'a [ObservablePolicy],
}
impl<'a> Policies<'a> {
    /// No allocation. At most `maximum_pair_checks` key comparisons are performed.
    pub fn new(
        entries: &'a [ObservablePolicy],
        maximum_pair_checks: usize,
    ) -> Result<Self, VerificationError> {
        if entries.is_empty() {
            return Err(VerificationError::MissingPolicy);
        }
        let mut remaining = maximum_pair_checks;
        for (index, entry) in entries.iter().enumerate() {
            for previous in &entries[..index] {
                if remaining == 0 {
                    return Err(VerificationError::CapacityExceeded);
                }
                remaining -= 1;
                if previous.key == entry.key {
                    return Err(VerificationError::DuplicateObservable);
                }
            }
        }
        Ok(Self { entries })
    }

    /// Stable review order; every entry must be measured at every declared fine time.
    pub fn as_slice(self) -> &'a [ObservablePolicy] {
        self.entries
    }
}
