#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub max_tokens:
        usize,

    pub used_tokens:
        usize,
}

impl TokenBudget {
    pub fn remaining(
        &self,
    ) -> usize {
        self.max_tokens
            .saturating_sub(
                self.used_tokens,
            )
    }
}
