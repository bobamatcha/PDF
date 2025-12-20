//! Layer Cake compliance architecture
//!
//! Layers are checked in order (bottom to top):
//! 1. Federal - baseline requirements (Lead paint, Fair Housing)
//! 2. State - statutory requirements
//! 3. Local - municipal ordinances (rent control, RLTO)
//! 4. Variable - user inputs (constrained by lower layers)

pub mod federal;
pub mod local;

pub use federal::check_federal_compliance;
pub use local::check_local_compliance;
