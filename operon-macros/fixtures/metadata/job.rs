pub const fn job_beta_meta() -> operon::schema::JobMetadata<1usize> {
    operon::schema::JobMetadata {
        id: "beta",
        dims: ["i"],
        spawn_dim: Some("j"),
    }
}
