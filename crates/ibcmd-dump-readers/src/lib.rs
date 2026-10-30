//! Independent readers of stored configuration records used by source export.
//!
//! Additional-index records and evidence-bound configuration properties do not
//! need an SQL connection, active export state, the CLI or a file writer. The
//! root exporter retains their former module paths through private aliases.
//! Their source literals remain covered by the physical-adapter policy guard.

pub mod additional_indexes;
pub mod configuration_properties_evidence;
