use uuid::Uuid;

pub struct FailoverPlan {
    pub failed_node:
        Uuid,

    pub replacement_node:
        Uuid,
}
