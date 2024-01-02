pub mod dest_project_gen;
pub mod errors;
pub mod grants;

#[cfg(feature = "juno")]
pub mod juno_comp_prefs;
#[cfg(feature = "juno")]
pub mod juno_dest_project_gen;

#[cfg(feature = "osmosis")]
pub mod osmosis_comp_prefs;
#[cfg(feature = "osmosis")]
pub mod osmosis_dest_project_gen;

#[cfg(feature = "migaloo")]
pub mod migaloo_comp_prefs;
#[cfg(feature = "migaloo")]
pub mod migaloo_dest_project_gen;

#[cfg(any(feature = "juno", feature = "migaloo", feature = "osmosis", feature = "sail"))]
pub mod sail_comp_prefs;
#[cfg(any(feature = "juno", feature = "migaloo", feature = "osmosis", feature = "sail"))]
pub mod sail_dest_project_gen;

#[cfg(test)]
mod tests;
