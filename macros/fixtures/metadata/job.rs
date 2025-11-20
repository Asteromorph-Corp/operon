pub const fn job_beta_meta() -> operon::schema_base::JobMetadata<1usize> {
    operon::schema_base::JobMetadata {
        id: "beta",
        dims: ["i"],
        spawn_dim: Some("j"),
    }
}
