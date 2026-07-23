// The judge crate as a library: the ADR-0001 judgment core, callable by
// both binaries — `judge` (the operator CLI, the rehearsal and
// hostile-network escape hatch) and `holdend` (the service, ADR-0003).
// The core is one pipeline; the binaries differ only in how a request
// arrives and how progress is told.

pub mod assemble;
pub mod core;
pub mod lane;
pub mod publish;
pub mod ruling;
pub mod spawn;
pub mod wire;
