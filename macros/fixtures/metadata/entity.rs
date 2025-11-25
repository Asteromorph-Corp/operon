pub const fn entity_a_meta() -> operon::schema::EntityMetadata<1usize, A> {
    operon::schema::EntityMetadata {
        id: "a",
        dims: ["i"],
        _phantom: std::marker::PhantomData,
    }
}
