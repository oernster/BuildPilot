//! The flight deck: the ordered list of operations (SRS 3.6). Order is list order; there is no
//! separate sort key to drift out of step with it.

use std::error::Error;
use std::fmt;

use super::operation::{Operation, OperationConfig, OperationId};

/// The ordered operations. Identities are unique within a deck.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlightDeck {
    operations: Vec<Operation>,
}

impl FlightDeck {
    /// A deck holding `operations` in the given order; refuses a repeated identity.
    pub fn new(operations: Vec<Operation>) -> Result<Self, DeckError> {
        let mut deck = Self::default();
        for operation in operations {
            deck.add(operation)?;
        }
        Ok(deck)
    }

    /// The operations, top row first.
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// True when the deck holds no operations (CFG-007).
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// The operation with `id`.
    pub fn get(&self, id: &OperationId) -> Option<&Operation> {
        self.operations
            .iter()
            .find(|operation| operation.id() == id)
    }

    /// The row index of `id`.
    pub fn position(&self, id: &OperationId) -> Option<usize> {
        self.operations
            .iter()
            .position(|operation| operation.id() == id)
    }

    /// Appends `operation` as the bottom row.
    pub fn add(&mut self, operation: Operation) -> Result<(), DeckError> {
        if self.get(operation.id()).is_some() {
            return Err(DeckError::DuplicateId(operation.id().clone()));
        }
        self.operations.push(operation);
        Ok(())
    }

    /// Replaces the configuration of the operation at `index` (as `position` answered it), keeping
    /// its identity and position. An index with no operation changes nothing.
    pub fn replace_config_at(&mut self, index: usize, config: OperationConfig) {
        if let Some(operation) = self.operations.get_mut(index) {
            operation.set_config(config);
        }
    }

    /// Removes `id` and returns it.
    pub fn remove(&mut self, id: &OperationId) -> Result<Operation, DeckError> {
        let index = self.index_of(id)?;
        Ok(self.operations.remove(index))
    }

    /// Moves `id` so that it ends up at row `target` (ROW-003). `target` past the last row is
    /// refused rather than clamped, since it means the caller's view of the deck is stale.
    pub fn move_to(&mut self, id: &OperationId, target: usize) -> Result<(), DeckError> {
        let index = self.index_of(id)?;
        if target >= self.operations.len() {
            return Err(DeckError::PositionOutOfRange {
                target,
                rows: self.operations.len(),
            });
        }
        let operation = self.operations.remove(index);
        self.operations.insert(target, operation);
        Ok(())
    }

    /// Moves `id` one row up (ROW-004). Answers false, changing nothing, when it is already top.
    pub fn move_up(&mut self, id: &OperationId) -> Result<bool, DeckError> {
        let index = self.index_of(id)?;
        if index == 0 {
            return Ok(false);
        }
        self.operations.swap(index, index - 1);
        Ok(true)
    }

    /// Moves `id` one row down (ROW-004). Answers false, changing nothing, when it is already
    /// bottom.
    pub fn move_down(&mut self, id: &OperationId) -> Result<bool, DeckError> {
        let index = self.index_of(id)?;
        if index + 1 == self.operations.len() {
            return Ok(false);
        }
        self.operations.swap(index, index + 1);
        Ok(true)
    }

    fn index_of(&self, id: &OperationId) -> Result<usize, DeckError> {
        self.position(id)
            .ok_or_else(|| DeckError::NotFound(id.clone()))
    }
}

/// Why a deck change was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeckError {
    /// An operation with this identity is already on the deck.
    DuplicateId(OperationId),
    /// No operation with this identity is on the deck.
    NotFound(OperationId),
    /// A move named a row that does not exist.
    PositionOutOfRange {
        /// The row asked for.
        target: usize,
        /// How many rows there are.
        rows: usize,
    },
}

impl fmt::Display for DeckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "An operation with identity {id} already exists."),
            Self::NotFound(id) => write!(f, "No operation with identity {id} is on the deck."),
            Self::PositionOutOfRange { target, rows } => {
                write!(f, "Cannot move to row {target}: the deck has {rows} rows.")
            }
        }
    }
}

impl Error for DeckError {}
