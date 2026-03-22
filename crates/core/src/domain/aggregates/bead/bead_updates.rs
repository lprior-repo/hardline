//! Bead update methods.

use chrono::Utc;

use crate::beads::{Description, DomainError, Title};

use super::{Bead, BeadError};

impl Bead {
    // ========================================================================
    // UPDATE METHODS
    // ========================================================================

    /// Update the bead title.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    /// Returns `BeadError::InvalidTitle` if title validation fails.
    pub fn update_title(&self, new_title: impl Into<String>) -> Result<Self, BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        let title = Title::new(new_title.into())
            .map_err(|e: DomainError| BeadError::InvalidTitle(e.to_string()))?;

        Ok(Self {
            title,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }

    /// Update the bead description.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    /// Returns `BeadError::InvalidDescription` if description validation fails.
    pub fn update_description(
        &self,
        new_description: Option<impl Into<String>>,
    ) -> Result<Self, BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        let description = new_description
            .map(|d| {
                Description::new(d.into())
                    .map_err(|e: DomainError| BeadError::InvalidDescription(e.to_string()))
            })
            .transpose()?;

        Ok(Self {
            description,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }

    /// Update both title and description.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    /// Returns `BeadError` if validation fails.
    pub fn update(
        &self,
        new_title: impl Into<String>,
        new_description: Option<impl Into<String>>,
    ) -> Result<Self, BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        let title = Title::new(new_title.into())
            .map_err(|e: DomainError| BeadError::InvalidTitle(e.to_string()))?;
        let description = new_description
            .map(|d| {
                Description::new(d.into())
                    .map_err(|e: DomainError| BeadError::InvalidDescription(e.to_string()))
            })
            .transpose()?;

        Ok(Self {
            title,
            description,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }
}
