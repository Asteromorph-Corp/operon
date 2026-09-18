mod trait_service;
mod trait_storage;

mod mod_core;
pub(super) use mod_core::*;

/// A trait method paired with the summary signature that documents it.
type DocumentedFn = (String, syn::TraitItemFn);
