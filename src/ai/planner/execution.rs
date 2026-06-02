use uuid::Uuid;

#[derive(
    Debug,
    Clone,
)]
pub struct AiExecutionPlan {
    pub execution_id:
        Uuid,

    pub reasoning_steps:
        Vec<String>,
}
