//! Profile-gated codecs for native bodies that are not metadata-family rows.

// The body-profile selection is `ibcmd-compiler-families`'s; the codecs
// here and the root keep naming it from this module.
pub use ibcmd_compiler_families::compiler::bodies::BodyProfileError;
pub(crate) use ibcmd_compiler_families::compiler::bodies::SelectedBodyProfile;

/// Profile-gated base-free codec for `Ext/CommandInterface.xml` native bodies:
/// `ibcmd-compiler-families`' `compiler::bodies::command_interface`.
pub mod command_interface {
    pub use ibcmd_compiler_families::compiler::bodies::command_interface::{
        CommandInterfaceBody, CommandInterfaceCodecError, CommandInterfaceCodecProfile,
        CommandInterfaceModel, CommandOrder, CommandPlacement, CommandPlacementMode,
        CommandReference, CommandVisibility, SubsystemVisibility, VisibilityValue,
        compile_command_interface, decode_command_interface,
    };
}
pub mod dcs;
// The 8.5 picture-value rewrite the DCS codec reads.
use crate::mssql_dump::dcs_2_21;
// The dynamic-list, chart and planner writers are `ibcmd-form-writer`'s,
// with the form writer that calls them, and the role-rights writer is
// `ibcmd-metadata-blob`'s, with its packer; `ibcmd-metadata-blob` re-exports
// the former, and they keep their paths here.
pub use ibcmd_metadata_blob::dynamic_list;
/// Profile-gated base-free codec for managed `Form.xml` native bodies:
/// `ibcmd-compiler`'s `compiler::bodies::form`.
pub mod form {
    pub use ibcmd_compiler::compiler::bodies::form::{
        ManagedFormBody, ManagedFormCodecError, ManagedFormCodecProfile, compile_managed_form,
        decode_managed_form,
    };
    // The transitional reader of historic fixtures, for the export and apply
    // adapters.
    pub(crate) use ibcmd_compiler::compiler::bodies::form::decode_compatible_managed_form;
}
pub mod form_chart {
    //! The chart writers, in `ibcmd-form-writer` (re-exported by
    //! `ibcmd-metadata-blob`): what the moxel writer reads.
    pub(crate) use ibcmd_metadata_blob::form_chart::{ChartHost, chart_value, gantt_chart_value};
}
/// Writer for the managed-form item records a native 8.3.27 body stores.
///
/// The code is the `ibcmd-form-native` crate; this module keeps its
/// crate-internal path `compiler::bodies::form_native` for the root.
pub mod form_native {
    pub(crate) use ibcmd_form_native::*;
}
pub use ibcmd_metadata_blob::form_planner;
/// Base-free serializers for the interface assets beside the command
/// interfaces: `ibcmd-compiler-families`' `compiler::bodies::interface_assets`.
pub mod interface_assets {
    pub use ibcmd_compiler_families::compiler::bodies::interface_assets::{
        ClientApplicationInterfaceModel, ClientApplicationNode, ClientApplicationPanelDef,
        HomePageWorkAreaItem, HomePageWorkAreaModel, StandaloneContentExtended,
        StandaloneContentModel,
    };
}
pub mod mxl;
pub(crate) mod mxl_native;
pub mod predefined;
pub mod rights;
pub use ibcmd_metadata_blob::role_rights_writer;
pub mod support;
pub mod template;
// `TemplateKind` and its codec error, without the DCS and MXL codecs that
// `template` compiles bodies through: `ibcmd-compiler`'s.
pub(crate) use ibcmd_compiler::compiler::bodies::template_kind;

// The writers' and codecs' tests that read the root (its exporter, its test
// scratch).
#[cfg(test)]
mod form_chart_root_tests;
#[cfg(test)]
mod form_root_tests;
#[cfg(test)]
mod role_rights_writer_root_tests;

#[cfg(test)]
mod tests {
    use crate::compiler::bodies::command_interface::CommandInterfaceCodecProfile;
    use crate::compiler::bodies::dcs::DcsCodecProfile;
    use crate::compiler::bodies::form::ManagedFormCodecProfile;
    use crate::compiler::bodies::mxl::MxlCodecProfile;
    use crate::compiler::bodies::predefined::PredefinedCodecProfile;
    use crate::compiler::bodies::rights::RightsCodecProfile;
    use crate::compiler::bodies::support::SupportPolicyProfile;
    use crate::compiler::bodies::template::TemplateCodecProfile;
    use ibcmd_core::artifact::ProfileId;

    #[test]
    fn bundled_profile_selects_only_the_evidenced_body_cohort() {
        let registry = crate::profile_registry::load_bundled_profile_registry().unwrap();
        let supported = registry
            .get(&ProfileId::parse("platform-8.3.27.1989").unwrap())
            .unwrap();
        assert!(RightsCodecProfile::from_effective(supported).is_ok());
        assert!(PredefinedCodecProfile::from_effective(supported).is_ok());
        assert!(CommandInterfaceCodecProfile::from_effective(supported).is_ok());
        assert!(ManagedFormCodecProfile::from_effective(supported).is_ok());
        assert!(MxlCodecProfile::from_effective(supported).is_ok());
        assert!(DcsCodecProfile::from_effective(supported).is_ok());
        assert!(TemplateCodecProfile::from_effective(supported).is_ok());
        assert!(SupportPolicyProfile::from_effective(supported).is_ok());

        for id in ["platform-8.3.24.1819", "platform-8.5.1.1150"] {
            let profile = registry.get(&ProfileId::parse(id).unwrap()).unwrap();
            assert!(RightsCodecProfile::from_effective(profile).is_err());
            assert!(PredefinedCodecProfile::from_effective(profile).is_err());
            assert!(CommandInterfaceCodecProfile::from_effective(profile).is_err());
            assert!(ManagedFormCodecProfile::from_effective(profile).is_err());
            assert!(MxlCodecProfile::from_effective(profile).is_err());
            assert!(DcsCodecProfile::from_effective(profile).is_err());
            assert!(TemplateCodecProfile::from_effective(profile).is_err());
            assert!(SupportPolicyProfile::from_effective(profile).is_err());
        }
    }
}
