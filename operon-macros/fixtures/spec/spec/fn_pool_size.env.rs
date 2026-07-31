fn pool_size(&self) -> usize {
    let v = ::std::env::var("JOB_CONCURRENCY")
        .expect("Environment variable `JOB_CONCURRENCY` not set for task concurrency")
        .parse::<usize>()
        .expect("Environment variable `JOB_CONCURRENCY` is not a valid concurrency value");
    assert!(v != 0, "Environment variable `JOB_CONCURRENCY` must not be zero for task concurrency");
    v
}
