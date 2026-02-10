pub const fn dimension_j_meta() -> operon::schema::DimensionMetadata<1usize> {
    operon::schema::DimensionMetadata {
        id: "j",
        deps: ["i"],
    }
}
