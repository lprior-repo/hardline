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

#[cfg(test)]
mod tests {
    use crate::domain::aggregates::bead::{Bead, BeadError};
    use crate::domain::identifiers::BeadId;

    fn create_test_bead() -> Bead {
        let id = BeadId::parse("bd-1").expect("valid id");
        Bead::new(id, "Test Bead", Some("Original description")).expect("bead created")
    }

    // -- update_title() --

    #[test]
    fn update_title_success() {
        let bead = create_test_bead();
        let updated = bead.update_title("New Title").expect("update ok");
        assert_eq!(updated.title.as_str(), "New Title");
    }

    #[test]
    fn update_title_empty_rejects() {
        let bead = create_test_bead();
        let result = bead.update_title("");
        assert!(matches!(result, Err(BeadError::InvalidTitle(_))));
    }

    #[test]
    fn update_title_whitespace_only_rejects() {
        let bead = create_test_bead();
        let result = bead.update_title("   ");
        assert!(matches!(result, Err(BeadError::InvalidTitle(_))));
    }

    #[test]
    fn update_title_on_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(bead.update_title("New"), Err(BeadError::CannotModifyClosed));
    }

    #[test]
    fn update_title_preserves_description() {
        let bead = create_test_bead();
        let updated = bead.update_title("New Title").expect("update ok");
        assert_eq!(
            updated.description.as_ref().map(|d| d.as_str().to_string()),
            Some("Original description".to_string())
        );
    }

    // -- update_description() --

    #[test]
    fn update_description_to_some() {
        let id = BeadId::parse("bd-2").expect("valid id");
        let bead = Bead::new(id, "Test", None::<String>).expect("ok");
        let updated = bead
            .update_description(Some("New description"))
            .expect("update ok");
        assert_eq!(
            updated.description.as_ref().map(|d| d.as_str().to_string()),
            Some("New description".to_string())
        );
    }

    #[test]
    fn update_description_to_none() {
        let bead = create_test_bead();
        let updated = bead.update_description(None::<String>).expect("update ok");
        assert!(updated.description.is_none());
    }

    #[test]
    fn update_description_on_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(
            bead.update_description(Some("New")),
            Err(BeadError::CannotModifyClosed)
        );
    }

    // -- update() (both title and description) --

    #[test]
    fn update_both_success() {
        let bead = create_test_bead();
        let updated = bead
            .update("New Title", Some("New description"))
            .expect("update ok");
        assert_eq!(updated.title.as_str(), "New Title");
        assert_eq!(
            updated.description.as_ref().map(|d| d.as_str().to_string()),
            Some("New description".to_string())
        );
    }

    #[test]
    fn update_both_with_none_description() {
        let bead = create_test_bead();
        let updated = bead.update("New Title", None::<String>).expect("update ok");
        assert_eq!(updated.title.as_str(), "New Title");
        assert!(updated.description.is_none());
    }

    #[test]
    fn update_both_invalid_title_rejects() {
        let bead = create_test_bead();
        let result = bead.update("", Some("description"));
        assert!(matches!(result, Err(BeadError::InvalidTitle(_))));
    }

    #[test]
    fn update_both_on_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(
            bead.update("New", Some("desc")),
            Err(BeadError::CannotModifyClosed)
        );
    }

    #[test]
    fn update_updates_timestamp() {
        let bead = create_test_bead();
        let updated = bead.update_title("New").expect("update ok");
        assert!(updated.updated_at >= updated.created_at);
    }
}
