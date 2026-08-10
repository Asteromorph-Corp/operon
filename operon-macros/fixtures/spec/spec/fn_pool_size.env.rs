fn pool_size(&self) -> usize {
    let v = ::std::env::var("BETA_WORKERS")
        .expect("Environment variable `BETA_WORKERS` not set for task concurrency")
        .parse::<usize>()
        .expect("Environment variable `BETA_WORKERS` is not a valid concurrency value");
    assert!(v != 0, "Environment variable `BETA_WORKERS` must not be zero for task concurrency");
    v
}
