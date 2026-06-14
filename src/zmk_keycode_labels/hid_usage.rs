use crate::layout_key::modifier_symbols;
use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use zmk_studio_api::{
    HidUsage, MOD_LALT, MOD_LCTL, MOD_LGUI, MOD_LSFT, MOD_RALT, MOD_RCTL, MOD_RGUI, MOD_RSFT,
};

use super::keycode_label::keycode_to_layout_key;

pub fn hid_usage_to_layout_key(usage: HidUsage) -> LayoutKey {
    if usage.modifiers() == 0 {
        if let Some(keycode) = usage.known_keycode() {
            return keycode_to_layout_key(&keycode);
        }

        return LayoutKey {
            tap: Label::new(format!("0x{:08X}", usage.to_hid_usage())),
            ..Default::default()
        };
    }

    if let Some(named_key) = usage.known_keycode() {
        return keycode_to_layout_key(&named_key);
    }

    let base = usage.base();

    // A lone shift over a key that has a shifted legend just produces that shifted
    // character (e.g. LS(N1) == "!"), so render it as a plain key.
    let mods = usage.modifiers();
    if mods & !(MOD_LSFT | MOD_RSFT) == 0 {
        if let Some(base_keycode) = base.known_keycode() {
            if let Some(shifted) = keycode_to_layout_key(&base_keycode).shifted {
                return LayoutKey {
                    tap: Label::new(shifted),
                    ..Default::default()
                };
            }
        }
    }

    // Otherwise show the base key in `tap` and the applied modifiers as glyphs in
    // the argument strip (e.g. "C" + "⎈" for LC(C)).
    let (tap, symbol) = if let Some(base_keycode) = base.known_keycode() {
        let base_key = keycode_to_layout_key(&base_keycode);
        (base_key.tap, base_key.symbol)
    } else {
        (Label::new(format!("0x{:08X}", base.to_hid_usage())), None)
    };

    LayoutKey {
        tap,
        argument: Some(modifier_symbols::glyphs(
            mods & (MOD_LCTL | MOD_RCTL) != 0,
            mods & (MOD_LSFT | MOD_RSFT) != 0,
            mods & (MOD_LALT | MOD_RALT) != 0,
            mods & (MOD_LGUI | MOD_RGUI) != 0,
        )),
        symbol,
        kind: KeycodeKind::Modifier,
        ..Default::default()
    }
}
