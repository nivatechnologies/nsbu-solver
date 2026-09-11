//! Complete physical refinements from actual exact-v2 accepted states.
mod extrema;
mod plan;
mod report;
mod workspace;
pub use extrema::{PhysicalExtrema, SampleMaximum};
use nsbu_solver::diagnostics::physical::PhysicalQuantity;
pub use plan::{PhysicalFamilyBounds, PhysicalFamilyPlan, PhysicalFamilyWork};
pub use report::{PhysicalRefinementSample, QuantityRefinement};
pub use workspace::PhysicalFamilyWorkspace;

/// Fixed velocity, gradient, Hessian and vorticity order.
pub const QUANTITIES: [PhysicalQuantity; 4] = [
    PhysicalQuantity::Vector,
    PhysicalQuantity::Gradient,
    PhysicalQuantity::Hessian,
    PhysicalQuantity::Vorticity,
];
