pub const fn dimension_j_meta() -> operon::schema_base::DimensionMetadata<1usize> {
    operon::schema_base::DimensionMetadata {
        id: "j",
        deps: ["i"],
    }
}
