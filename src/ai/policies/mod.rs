
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts:
        usize,
}

#[derive(Debug, Clone)]
pub struct RoutingPolicy {
    pub allow_fallback:
        bool,
}
