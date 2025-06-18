operon_macros::include_operon! {
    config = {
        storage.data.uri = "postgres://user:password@hostname:port/operon-db",
        storage.data.pool_size = 16,
        storage.data.schema = "data",
        storage.metadata.uri = "postgres://user:password@hostname:port/operon-db",
        storage.data.schema = "metadata",
        log.level = "debug",
        log.buffer_size = 1024,
        log.dump = true,
        log.dump_path = "logs",
    };
    types = {
        #[entity(primary, dims = ["i"])]
        pub struct A(pub String);

        #[entity(dims = ["i", "j"], def = "beta|i", from = ["A"], pool = 8)]
        pub struct B(pub A, pub usize);

        #[entity(dims = ["i", "k"], def = "gamma|i", from = ["A"], pool = 8)]
        pub struct C(pub usize);

        #[entity(
            dims = ["i", "j", "k"],
            def = "delta | i, j, k",
            from = ["A", "B", "C"],
            pool = 4
        )]
        pub struct D {
            pub a: A,
            pub b: B,
            pub c: C,
        };

        #[entity(dims = ["i", "k"], def = "epsilon | i, k", from = ["B | j", "D|j"], pool = 4)]
        pub struct E {
            pub b: Vec<B>,
            pub d: Vec<D>,
        };

        #[entity(dims = ["i"], def = "zeta|i", from = ["C|k", "E|k"])]
        pub enum F {
            Success {
                c: Vec<C>,
                e: Vec<E>,
            },
            Failure(String, Option<C>, Option<E>),
        };
    };
    use_psql_storage = DataStorage;
}
