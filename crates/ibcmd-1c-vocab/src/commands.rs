//! Command shortcuts: the `{0,<virtual key>,<modifier mask>}` tuple and the
//! spelling the platform writes for it.

use ibcmd_1c_text::split_1c_braced_fields;

/// Names the platform writes for the shortcut bitmask's CTRL and ALT bits.
///
/// The bits themselves are host-neutral -- the same `.cf` carries the same
/// `{0, virtual-key, modifier-mask}` tuple everywhere -- but `ibcmd` spells
/// them for the host it runs on, and that is the only thing that changes:
/// bit 8 is `Ctrl` on Windows and `Cmd` on macOS, bit 16 is `Alt` on Windows
/// and `Option` on macOS. Bit 4 stays `Shift`, the key name never moves, and
/// the modifier order is the same on both.
///
/// Measured on БСП базовая: of the 81 forms carrying `<Shortcut>`, 50 are
/// already byte-identical to the macOS reference tree and the other 31 become
/// byte-identical under exactly this substitution, with no residual
/// difference anywhere. The stand's reference trees are macOS captures
/// throughout -- 2 900 `Cmd` and 140 `Option` across uh/ut/ssl/sslbase, and
/// not one `Ctrl` or `Alt` -- so on this host the macOS spelling is what the
/// platform bytes say.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ShortcutModifierStyle {
    Windows,
    Macos,
}

impl ShortcutModifierStyle {
    /// The spelling `ibcmd` uses on the host this export is running on.
    pub const fn host() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Windows
        }
    }

    pub const fn ctrl(self) -> &'static str {
        match self {
            Self::Windows => "Ctrl",
            Self::Macos => "Cmd",
        }
    }

    pub const fn alt(self) -> &'static str {
        match self {
            Self::Windows => "Alt",
            Self::Macos => "Option",
        }
    }
}

pub fn parse_common_command_shortcut_value(value: &str) -> Option<String> {
    parse_common_command_shortcut_value_with_style(value, ShortcutModifierStyle::host())
}

pub fn parse_common_command_shortcut_value_with_style(
    value: &str,
    style: ShortcutModifierStyle,
) -> Option<String> {
    let fields = split_1c_braced_fields(value, 0)?;
    if fields.len() != 3 || fields.first()?.trim() != "0" {
        return None;
    }
    let key_code = fields.get(1)?.trim().parse::<u16>().ok()?;
    let modifier_code = fields.get(2)?.trim().parse::<u16>().ok()?;
    const SHIFT: u16 = 4;
    const CTRL: u16 = 8;
    const ALT: u16 = 16;
    if key_code == 0 || modifier_code & !(SHIFT | CTRL | ALT) != 0 {
        return None;
    }
    let key = match key_code {
        8 => "BackSpace".to_string(),
        13 => "Enter".to_string(),
        48..=57 => char::from_u32(u32::from(key_code))?.to_string(),
        65..=90 => char::from_u32(u32::from(key_code))?.to_string(),
        96..=105 => format!("Num {}", key_code - 96),
        // 106..=111 continue the VK_NUMPAD0..VK_DIVIDE numpad run 96..=105
        // already maps (VK_MULTIPLY=106, VK_ADD=107, VK_SEPARATOR=108,
        // VK_SUBTRACT=109, VK_DECIMAL=110 already spelled `Num .`,
        // VK_DIVIDE=111). Evidenced: `ssl`-demo's `DataProcessors/
        // КонсольЗапросов/Forms/Форма` carries key_code 111 twice (modifier 8
        // and 12), native `Cmd+Num /` and `Cmd+Shift+Num /`.
        // VK_ADD=107 and VK_SUBTRACT=109, the two of the 106..=109 run the
        // stand spells. ERP УХ 3.2.12.6 `Reports/ДеревоСебестоимостиПродукции/
        // Forms/ФормаОтчета` and
        // `Reports/СтруктураЗаказаНаПроизводствоДинамическая/Forms/ФормаОтчета`
        // each carry `{0,107,12}` and `{0,109,12}` on their expand-all and
        // collapse-all commands, and the platform writes `Cmd+Shift+Num +` and
        // `Cmd+Shift+Num -` for them. VK_MULTIPLY=106 and VK_SEPARATOR=108
        // occur nowhere in the eight trees and stay unread rather than guessed
        // from the run.
        //
        // VK_MULTIPLY=106 does occur: the eight native stand trees spell
        // `<Shortcut>Num *</Shortcut>` twice, on the `КнопкаУмножить` command
        // of ERP УХ 3.2.12.6
        // `DataProcessors/НастройкаСтруктурыОтчета/Forms/ФормаКонструктора`
        // and of `Catalogs/БланкиОтчетов/Forms/ФормаМакета`, so the census
        // that declared the code absent had not seen them. Seed `hg7-a`
        // against 8.3.27.2214 measures the code itself: a form command written
        // `<Shortcut>Num *</Shortcut>` stores `{0,106,0}` and its sibling
        // written `<Shortcut>Num /</Shortcut>` stores `{0,111,0}`, the code
        // this run already spells. VK_SEPARATOR=108 is spelled by no tree and
        // no seed and stays unread.
        106 => "Num *".to_string(),
        107 => "Num +".to_string(),
        109 => "Num -".to_string(),
        110 => "Num .".to_string(),
        111 => "Num /".to_string(),
        112..=123 => format!("F{}", key_code - 111),
        // VK_ESCAPE. The one unmapped key code any of the three probed
        // configurations spells: Документооборот КОРП 3.0.21.3 carries it once
        // with no modifier, on the very command the platform writes
        // `<Shortcut>Esc</Shortcut>` for -- the only `Esc` in the eight native
        // stand trees -- and УТ 11.5.27.75 spells no unmapped code at all.
        27 => "Esc".to_string(),
        _ => return None,
    };
    let mut parts = Vec::with_capacity(4);
    if modifier_code & CTRL != 0 {
        parts.push(style.ctrl().to_string());
    }
    if modifier_code & ALT != 0 {
        parts.push(style.alt().to_string());
    }
    if modifier_code & SHIFT != 0 {
        parts.push("Shift".to_string());
    }
    parts.push(key);
    Some(parts.join("+"))
}
