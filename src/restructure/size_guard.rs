//! Compatibility exports and the root settings/apply gate adapters.

use crate::mssql_config_apply::gate::{GateVerdict, StructurePhase};
use crate::restructure::plan::Method;
use crate::restructure::reader::RowSource;
use crate::settings::Settings;
use anyhow::{Context, Result, bail};
pub use ibcmd_restructure_engine::restructure::size_guard::*;

/// The limit a command runs with: the flag, else the settings chain (variable, settings file), else the
/// default; rows and bytes each on their own. A limit of 0 is refused: it would refuse every rebuild.
pub fn resolve_limit(
    settings: &Settings,
    flag_rows: Option<u64>,
    flag_bytes: Option<&str>,
) -> Result<LimitSetting> {
    let default = RestructureLimit::default();
    let (rows, rows_source) = match flag_rows {
        Some(rows) => (rows, "--restructure-limit-rows".to_owned()),
        None => match settings.restructure_limit_rows()? {
            Some(setting) => (setting.value, setting.source.to_string()),
            None => (default.rows, DEFAULT_SOURCE.to_owned()),
        },
    };
    let (bytes, bytes_source) = match flag_bytes {
        Some(text) => (
            parse_byte_size(text).context("--restructure-limit-bytes")?,
            "--restructure-limit-bytes".to_owned(),
        ),
        None => match settings.restructure_limit_bytes()? {
            Some(setting) => (setting.value, setting.source.to_string()),
            None => (default.bytes, DEFAULT_SOURCE.to_owned()),
        },
    };
    if rows == 0 || bytes == 0 {
        bail!(
            "a restructure limit of 0 would refuse every rebuild (rows {rows} from {rows_source}, bytes {bytes} from {bytes_source})"
        );
    }
    Ok(LimitSetting {
        limit: RestructureLimit { rows, bytes },
        rows_source,
        bytes_source,
    })
}

/// The guard of a structure phase the gate has prepared: reads the sizes of the tables it rebuilds, puts
/// the verdict into the phase's report (`size_check`), and above the limit blocks the stage (the reason
/// starts with `S1:`, like every reason of the S1 gate) and drops the phase. Only `Method::Rebuild` copies
/// tables; the research `AlterAdd` adds a column in place and is not guarded. No phase, nothing to guard.
pub fn guard_phase(
    source: &mut dyn RowSource,
    method: Method,
    setting: &LimitSetting,
    verdict: &mut GateVerdict,
    phase: &mut Option<StructurePhase>,
) -> Result<()> {
    if method != Method::Rebuild {
        return Ok(());
    }
    let Some(prepared) = phase.as_mut() else {
        return Ok(());
    };
    let check = check_tables(source, &prepared.tables, setting)?;
    prepared.size_check = Some(check.to_json());
    if let Some(reason) = check.refusal() {
        verdict.block("", format!("S1: {reason}"));
        *phase = None;
    }
    Ok(())
}
