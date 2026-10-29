//! Compatibility exports of the native restructuring planner.

pub(crate) use ibcmd_restructure_engine::restructure::plan::indexing_word;
pub use ibcmd_restructure_engine::restructure::plan::{
    Addition, AlterColumn, CacheUpdate, CreatedSection, IndexSwitch, Inputs, Method, ObjectPlan,
    PROMOTE_CONFIG, Phase, Plan, PlanOptions, Removal, SectionPlan, StagedImage, Statement,
    StatementParam, TablePlan, Widening, configuration_uuid, parse_deleted, plan,
};
