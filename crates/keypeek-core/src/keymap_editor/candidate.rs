//! Candidate key models and search indexing for the keymap editor.

use crate::key_presenter::KeyPresenter;
use crate::key_spec::KeySpec;
use crate::layout_key::{Label, LayoutKey};

/// Candidate key binding displayed in a picker grid.
#[derive(Clone)]
pub struct Candidate {
    /// Key specification.
    pub binding: KeySpec,
    /// Visual key representation.
    pub key: LayoutKey,
    /// Indicates a transparent key slot.
    pub transparent: bool,
    /// Precomputed lowercase search tokens.
    search_haystack: String,
}

impl Candidate {
    pub fn new(binding: KeySpec, key: LayoutKey) -> Self {
        let search_haystack = build_search_haystack(&binding, &key);
        Self {
            binding,
            key,
            transparent: false,
            search_haystack,
        }
    }

    /// Sets whether this candidate represents a transparent key slot.
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Appends a search token to the search haystack.
    pub fn with_search_token(mut self, token: impl AsRef<str>) -> Self {
        push_token(&mut self.search_haystack, token.as_ref());
        self
    }

    /// Appends search tokens to the search haystack.
    pub fn with_search_tokens(mut self, tokens: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        for token in tokens {
            push_token(&mut self.search_haystack, token.as_ref());
        }
        self
    }

    /// Checks whether this candidate matches the search query.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        self.matches_lowercased(&q)
    }

    /// Fast search check against pre-lowercased query token without allocations.
    pub fn matches_lowercased(&self, lowercase_query: &str) -> bool {
        self.search_haystack.contains(lowercase_query)
    }

    /// Creates a candidate for any key action, providing consistent display
    /// labels for transparent and none slots across protocols.
    pub fn from_action(
        binding: KeySpec,
        presenter: &dyn KeyPresenter,
        layer_names: &[String],
    ) -> Self {
        let is_none = matches!(binding, KeySpec::None);
        if is_none {
            return Self::new(
                binding,
                LayoutKey {
                    tap: Label::new("None"),
                    ..Default::default()
                },
            );
        }

        match presenter.present_key(&binding, layer_names) {
            None => Self::new(
                binding,
                LayoutKey {
                    tap: Label::with_short("Transparent", egui_phosphor::regular::CARET_DOWN),
                    ..Default::default()
                },
            )
            .with_transparent(true),
            Some(key) => Self::new(binding, key),
        }
    }
}

fn push_token(haystack: &mut String, s: &str) {
    for c in s.chars().flat_map(char::to_lowercase) {
        haystack.push(c);
    }
    haystack.push(' ');
}

fn push_opt_token(haystack: &mut String, opt: &Option<String>) {
    if let Some(s) = opt {
        push_token(haystack, s);
    }
}

fn push_lbl_token(haystack: &mut String, lbl: &Option<Label>) {
    if let Some(l) = lbl {
        push_token(haystack, &l.full);
        push_opt_token(haystack, &l.short);
    }
}

fn build_search_haystack(binding: &KeySpec, key: &LayoutKey) -> String {
    let mut haystack = String::with_capacity(64);

    push_token(&mut haystack, &key.tap.full);
    push_opt_token(&mut haystack, &key.tap.short);
    push_opt_token(&mut haystack, &key.shifted);
    push_opt_token(&mut haystack, &key.ralt);
    push_opt_token(&mut haystack, &key.ralt_shifted);
    push_opt_token(&mut haystack, &key.symbol);
    push_lbl_token(&mut haystack, &key.behavior);
    push_lbl_token(&mut haystack, &key.argument);

    match binding {
        KeySpec::KeyPress { key, .. }
        | KeySpec::KeyToggle { key, .. }
        | KeySpec::LayerTap { tap: key, .. }
        | KeySpec::ModTap { tap: key, .. }
        | KeySpec::StickyKey { key: Some(key), .. } => {
            use std::fmt::Write;
            let _ = write!(&mut haystack, "{:04x} ", key.id);
        }
        _ => {}
    }

    haystack
}

/// Selected key state and validity indicator for picker grids.
#[derive(Clone, Copy)]
pub struct SelectedKey<'a> {
    pub action: &'a KeySpec,
    pub valid: bool,
}

impl<'a> SelectedKey<'a> {
    pub fn valid(action: &'a KeySpec) -> Self {
        Self {
            action,
            valid: true,
        }
    }

    pub fn new(action: &'a KeySpec, valid: bool) -> Self {
        Self { action, valid }
    }
}

impl<'a> From<&'a KeySpec> for SelectedKey<'a> {
    fn from(action: &'a KeySpec) -> Self {
        Self::valid(action)
    }
}

/// A candidate group for the key picker grid.
#[derive(Clone)]
pub struct CandidateGroup {
    pub name: &'static str,
    pub candidates: Vec<Candidate>,
}

impl CandidateGroup {
    pub fn new(name: &'static str, candidates: Vec<Candidate>) -> Self {
        Self { name, candidates }
    }
}
