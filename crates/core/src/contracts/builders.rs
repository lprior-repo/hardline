//! Builder types for constructing contracts

use im::HashMap;

use super::types::{Constraint, ContextualHint, FieldContract, TypeContract};

// ═══════════════════════════════════════════════════════════════════════════
// BUILDERS
// ═══════════════════════════════════════════════════════════════════════════

pub struct TypeContractBuilder {
    pub name: String,
    pub description: String,
    pub constraints: Vec<Constraint>,
    pub hints: Vec<ContextualHint>,
    pub examples: Vec<String>,
    pub fields: HashMap<String, FieldContract>,
}

impl TypeContractBuilder {
    /// Set the description for the contract.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a constraint to the contract.
    #[must_use]
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add a contextual hint to the contract.
    #[must_use]
    pub fn hint(mut self, hint: ContextualHint) -> Self {
        self.hints.push(hint);
        self
    }

    /// Add an example to the contract.
    #[must_use]
    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Add a field to the contract.
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, field: FieldContract) -> Self {
        self.fields = self.fields.update(name.into(), field);
        self
    }

    /// Build the final `TypeContract`.
    ///
    /// # Returns
    ///
    /// Returns the constructed contract. The result must be used as this
    /// consumes the builder.
    #[must_use]
    pub fn build(self) -> TypeContract {
        TypeContract {
            name: self.name,
            description: self.description,
            constraints: self.constraints,
            hints: self.hints,
            examples: self.examples,
            fields: self.fields,
        }
    }
}

pub struct FieldContractBuilder {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub description: String,
    pub constraints: Vec<Constraint>,
    pub default: Option<String>,
    pub depends_on: Vec<String>,
    pub examples: Vec<String>,
}

impl FieldContractBuilder {
    /// Mark the field as required.
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set the field description.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a constraint to the field.
    #[must_use]
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set the default value for the field.
    #[must_use]
    pub fn default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Add a dependency on another field.
    #[must_use]
    pub fn depends_on(mut self, field: impl Into<String>) -> Self {
        self.depends_on.push(field.into());
        self
    }

    #[must_use]
    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Build the final `FieldContract`.
    ///
    /// # Returns
    ///
    /// Returns the constructed contract. The result must be used as this
    /// consumes the builder.
    #[must_use]
    pub fn build(self) -> FieldContract {
        FieldContract {
            name: self.name,
            field_type: self.field_type,
            required: self.required,
            description: self.description,
            constraints: self.constraints,
            default: self.default,
            depends_on: self.depends_on,
            examples: self.examples,
        }
    }
}
